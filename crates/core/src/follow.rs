//! File tailing for follow mode (`--follow`).
//!
//! This module owns the low-level mechanics of watching a growing file:
//! tracking the read offset, buffering partial lines, and detecting the
//! two conditions that require re-opening the file:
//!
//! - **Rotation**: the file at the watched path has been replaced by a new
//!   file. Detected by comparing `(dev, ino)` before and after each read
//!   via [`std::os::unix::fs::MetadataExt`]. When the identity changes,
//!   the old handle is closed, the path is re-opened, and the offset resets
//!   to 0.
//!
//! - **Truncation**: the on-disk file size is smaller than the tracked
//!   offset — a `>` redirect or `truncate(1)` was used in-place. On
//!   detection, the offset resets to 0 and reading continues from the
//!   beginning of the (now-shorter) file.
//!
//! Both conditions are checked on every call to [`FileTailer::read_new_lines`].
//!
//! # Tail-from-EOF semantics
//!
//! [`FileTailer::open`] seeks to the current end-of-file immediately. The
//! first call to [`read_new_lines`] therefore returns only bytes appended
//! *after* `open` was called, matching `tail -f` behaviour.
//!
//! # Partial line buffering
//!
//! Bytes that do not yet end with a newline are kept in `leftover` until
//! subsequent reads complete the line. Both `\n` and `\r\n` are stripped.
//!
//! # Thread safety
//!
//! `FileTailer` is single-threaded by design. The CLI watch loop owns the
//! instance exclusively. No `Mutex` is needed.
//!
//! # Unix-only
//!
//! `(dev, ino)` rotation detection requires `std::os::unix::fs::MetadataExt`.
//! The module is gated with `#![cfg(unix)]`; Windows support is deferred to
//! v0.3.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::error::{LogdiveError, Result};

/// Read buffer for each [`FileTailer::read_new_lines`] call.
///
/// 8 KiB is a common sweet-spot: large enough to amortise syscall overhead
/// for typical log line rates, small enough to keep stack pressure low.
const READ_BUFFER_SIZE: usize = 8 * 1024;

/// Tracks a growing file, yielding newly appended complete lines on each
/// call to [`read_new_lines`].
///
/// See the module-level documentation for the full semantics.
#[derive(Debug)]
pub struct FileTailer {
    /// Watched path. Re-opened on rotation.
    path: PathBuf,
    /// Open handle to the file at `path` at construction or last rotation.
    file: File,
    /// Byte offset of the next read inside the current file handle.
    offset: u64,
    /// inode number of the open file handle, used for rotation detection.
    inode: u64,
    /// Device number of the open file handle, used for rotation detection.
    dev: u64,
    /// Bytes of an incomplete trailing line carried over between reads.
    leftover: Vec<u8>,
}

impl FileTailer {
    /// Open the file at `path` and seek to its current end.
    ///
    /// Subsequent calls to [`read_new_lines`] return only data appended
    /// after this point — identical to `tail -f` startup behaviour.
    ///
    /// Returns `Err` if the file does not exist or cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path).map_err(|e| LogdiveError::io_at(&path, e))?;

        let meta = file.metadata().map_err(|e| LogdiveError::io_at(&path, e))?;
        let inode = meta.ino();
        let dev = meta.dev();
        let offset = meta.len();

        // Seek to EOF so we only return bytes appended after open().
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| LogdiveError::io_at(&path, e))?;

        Ok(Self {
            path,
            file,
            offset,
            inode,
            dev,
            leftover: Vec::new(),
        })
    }

    /// Read any newly appended bytes, split into complete lines, and return
    /// them. A partial trailing line is buffered until it is terminated.
    ///
    /// Both `\n` and `\r\n` line endings are stripped. Invalid UTF-8 bytes
    /// are replaced with U+FFFD via [`String::from_utf8_lossy`].
    ///
    /// Rotation and truncation are checked before every read. If the path
    /// briefly disappears during a rotation (the window between the old
    /// file being renamed away and the new one being created), this method
    /// returns `Ok(vec![])` and retries on the next call.
    pub fn read_new_lines(&mut self) -> Result<Vec<String>> {
        // --- Rotation / truncation check -----------------------------------
        match std::fs::metadata(&self.path) {
            Ok(meta) => {
                let current_ino = meta.ino();
                let current_dev = meta.dev();
                let current_size = meta.len();

                if current_ino != self.inode || current_dev != self.dev {
                    // The file at the path is a different inode: rotation.
                    self.handle_rotation()?;
                } else if current_size < self.offset {
                    // Same inode but size shrank: truncation.
                    self.offset = 0;
                    self.leftover.clear();
                    self.file
                        .seek(SeekFrom::Start(0))
                        .map_err(|e| LogdiveError::io_at(&self.path, e))?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Path is momentarily absent (mid-rotation gap). Return
                // empty and let the caller retry on the next event.
                return Ok(vec![]);
            }
            Err(e) => return Err(LogdiveError::io_at(&self.path, e)),
        }

        // --- Read new bytes -------------------------------------------------
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut raw_bytes: Vec<u8> = Vec::new();

        loop {
            match self.file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    raw_bytes.extend_from_slice(&buf[..n]);
                    self.offset += n as u64;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(LogdiveError::io_at(&self.path, e)),
            }
        }

        if raw_bytes.is_empty() && self.leftover.is_empty() {
            return Ok(vec![]);
        }

        // Prepend any leftover bytes from the previous call.
        let mut combined = std::mem::take(&mut self.leftover);
        combined.extend_from_slice(&raw_bytes);

        // --- Split into lines -----------------------------------------------
        let mut lines: Vec<String> = Vec::new();
        let mut start = 0usize;

        while start < combined.len() {
            // Find the next newline.
            match combined[start..].iter().position(|&b| b == b'\n') {
                Some(rel) => {
                    let end = start + rel;
                    // Slice the line bytes, stripping \r if present.
                    let line_bytes = if end > start && combined[end - 1] == b'\r' {
                        &combined[start..end - 1]
                    } else {
                        &combined[start..end]
                    };
                    let line = String::from_utf8_lossy(line_bytes).into_owned();
                    lines.push(line);
                    start = end + 1; // skip the \n
                }
                None => {
                    // Remainder is a partial line — buffer it.
                    self.leftover = combined[start..].to_vec();
                    return Ok(lines);
                }
            }
        }

        Ok(lines)
    }

