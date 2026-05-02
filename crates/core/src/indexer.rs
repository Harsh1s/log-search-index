//! SQLite-backed index for ingested log entries.
//!
//! This module owns the persistent storage side of logdive: schema creation,
//! row-level deduplication via `blake3`, batched inserts of 1000 rows per
//! transaction (per the decisions log entry dated 2026-04-19), and time-based
//! retention via [`Indexer::prune`]. The schema is reproduced verbatim from
//! the project doc's "SQLite schema" section with `IF NOT EXISTS` added so
//! opening an existing database is idempotent.
//!
//! `Indexer` is an owning handle over a `rusqlite::Connection`. It can be
//! constructed against a filesystem path via [`Indexer::open`] or against an
//! in-memory database via [`Indexer::open_in_memory`] — the latter is used
//! by the unit tests below and will also serve ad-hoc one-shot scenarios.
//! For read-only consumers (the HTTP API in milestone 8), [`Indexer::
//! open_read_only`] opens an existing database without the schema init or
//! directory-creation side effects of [`Indexer::open`], and enforces
//! read-only semantics at the SQLite level via `SQLITE_OPEN_READ_ONLY`.
//!
//! # Timestamp NOT NULL policy
//!
//! The schema declares `timestamp TEXT NOT NULL`, but the parser produces
//! `LogEntry::timestamp = None` for lines that omit the key. Rather than
//! fabricating a fallback (which would falsely anchor those rows to
//! ingestion time and confuse `last Nh` queries), the indexer *skips* such
//! rows and reports them in [`InsertStats::skipped_no_timestamp`]. This
//! mirrors the parser's "graceful skip" philosophy — bad data is counted
//! and dropped, never manufactured.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, params};

use crate::entry::LogEntry;
use crate::error::{LogdiveError, Result};

/// Size of a single insert transaction, per the decisions log
/// (2026-04-19: "batch insert per 1000 lines").
pub const BATCH_SIZE: usize = 1000;

const DEFAULT_DB_FILENAME: &str = "index.db";
const LOGDIVE_HOME_DIRNAME: &str = ".logdive";

/// Resolve the path to the index database.
///
/// When `override_path` is `Some`, it is used verbatim — this is what the
/// CLI's `--db` flag wires into. Otherwise the default `~/.logdive/index.db`
/// is returned per the "Default index location" decision in the project doc.
///
/// Purely functional: does not touch the filesystem.
pub fn db_path(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    // POSIX-centric: logdive's Phase 4 release targets are Linux and macOS,
    // both of which expose HOME. Fall back to CWD if it is unset (containers,
    // stripped CI environments) rather than panicking.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(LOGDIVE_HOME_DIRNAME)
        .join(DEFAULT_DB_FILENAME)
}

/// Outcome of an insert batch, surfaced to the CLI for progress output
/// ("lines ingested / lines skipped per second", per milestone 6).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InsertStats {
    /// Rows newly added to the index.
    pub inserted: usize,
    /// Rows rejected by `INSERT OR IGNORE` because their `raw_hash` already
    /// existed — the dedup path per the decisions log.
    pub deduplicated: usize,
    /// Rows rejected because they had no `timestamp`. See module docs.
    pub skipped_no_timestamp: usize,
}

impl InsertStats {
    fn extend(&mut self, other: InsertStats) {
        self.inserted += other.inserted;
        self.deduplicated += other.deduplicated;
        self.skipped_no_timestamp += other.skipped_no_timestamp;
    }
}

/// Outcome of a [`Indexer::prune`] operation, surfaced to the CLI's `prune`
/// subcommand for its completion summary.
///
/// Marked `#[non_exhaustive]` so later milestones can add fields (e.g. bytes
/// reclaimed by the `VACUUM`) without breaking the public API.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PruneStats {
    /// Number of rows deleted by the prune.
    pub deleted: u64,
}

/// Aggregate metadata about the contents of an index.
///
/// Produced by [`Indexer::stats`] and consumed by the CLI `stats` subcommand
/// (milestone 7) and the `GET /stats` HTTP endpoint (milestone 8). The shape
/// is intentionally minimal and structural; the CLI and HTTP layers format
/// it for human or machine consumption.
///
/// `tags` ordering: `None` (untagged rows) first, then non-null tag strings
/// in ascending alphabetical order. This ordering is produced directly by
/// SQLite (`ORDER BY tag` places NULL first in ascending order) and is not
/// re-sorted in Rust. The CLI renders the `None` slot as "(untagged)".
///
/// Marked `#[non_exhaustive]` so additional summary fields (e.g. distinct
/// level counts) can be added in later milestones without breaking the
/// public API.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Stats {
    /// Total number of rows currently in the `log_entries` table.
    pub entries: u64,
    /// Lexically smallest `timestamp` value in the index, or `None` on an
    /// empty database. Lexical ordering is correct for ISO-8601 timestamps;
    /// see the "live design decisions" section of the project handoff.
    pub min_timestamp: Option<String>,
    /// Lexically largest `timestamp` value in the index, or `None` on an
    /// empty database.
    pub max_timestamp: Option<String>,
    /// Distinct tag values observed across all rows. `None` represents rows
    /// with no tag (SQL NULL) and — when present — is always the first
    /// element; non-null tags follow in ascending alphabetical order.
    pub tags: Vec<Option<String>>,
}

/// Owning handle over a SQLite connection to a logdive index.
#[derive(Debug)]
pub struct Indexer {
    conn: Connection,
}

impl Indexer {
    /// Open (or create) a logdive index at `path`.
    ///
    /// Creates the parent directory if it does not already exist, opens the
    /// SQLite database, and runs idempotent schema migrations.
    pub fn open(path: &Path) -> Result<Self> {
        ensure_parent_dir(path)?;
        let conn = Connection::open(path)?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory index. Used by tests; also usable for one-shot
    /// scenarios that don't need persistence.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an existing logdive index at `path` in read-only mode.
    ///
    /// Unlike [`Indexer::open`], this method:
    ///   1. Does **not** create the database file if it is missing (the
    ///      `SQLITE_OPEN_READ_ONLY` flag fails rather than creates),
    ///   2. Does **not** create the parent directory,
    ///   3. Does **not** run schema migrations — the caller is promising
    ///      that `path` already points at a valid logdive index.
    ///
    /// Enforcement of read-only semantics is at the SQLite level: any
    /// attempted write through the returned connection raises a runtime
    /// error. This is defense-in-depth for the HTTP API (milestone 8),
    /// whose surface is exclusively read.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        // `SQLITE_OPEN_URI` is included because it's the safe default
        // documented by rusqlite; it only affects parsing of `file:...`
        // URIs, which we never pass in.
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let conn = Connection::open_with_flags(path, flags)?;
        Ok(Self { conn })
    }

    /// Borrow the underlying connection.
    ///
    /// Exposed so the query executor can run reads without an extra
    /// abstraction layer. Read-only borrow keeps ingestion and querying
    /// from contending over `&mut`.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Insert a slice of entries into the index, chunking internally into
    /// transactions of [`BATCH_SIZE`] rows each.
    ///
    /// Returns aggregate stats across all chunks. Entry ordering within
    /// the index is not guaranteed.
    pub fn insert_batch(&mut self, entries: &[LogEntry]) -> Result<InsertStats> {
        let mut total = InsertStats::default();
        for chunk in entries.chunks(BATCH_SIZE) {
            let stats = insert_one_chunk(&mut self.conn, chunk)?;
            total.extend(stats);
        }
        Ok(total)
    }

    /// Delete every entry whose `timestamp` is strictly older than `cutoff`,
    /// then `VACUUM` to reclaim the freed disk space.
    ///
    /// `cutoff` is compared lexically against the stored `timestamp` TEXT
    /// column. This is correct for ISO-8601 / RFC3339 timestamps, which sort
    /// chronologically as text — the same comparison contract the query
    /// executor's `last` / `since` clauses rely on. A non-ISO-shaped cutoff
    /// (or non-ISO timestamps in the index) will compare incorrectly, the
    /// same known limitation that applies to time-range queries.
    ///
    /// The comparison is strict `<`: a row whose timestamp exactly equals
    /// `cutoff` is **kept**, not deleted.
    ///
    /// Returns the number of rows deleted in [`PruneStats::deleted`].
    ///
    /// # VACUUM and transactions
    ///
    /// SQLite refuses to run `VACUUM` inside an explicit transaction, so this
    /// method issues the `DELETE` and the `VACUUM` as two separate autocommit
    /// statements rather than wrapping them in `conn.transaction()`. The
    /// `DELETE` is a single statement and therefore atomic on its own; a
    /// crash between the two would leave the rows deleted but the file not
    /// yet compacted — harmless, since any later `VACUUM` reclaims the space.
    pub fn prune(&mut self, cutoff: &str) -> Result<PruneStats> {
        let deleted = self.conn.execute(
            "DELETE FROM log_entries WHERE timestamp < ?1",
            params![cutoff],
        )?;
        // VACUUM cannot run inside a transaction — issue it on its own.
        self.conn.execute_batch("VACUUM")?;
        Ok(PruneStats {
            deleted: deleted as u64,
        })
    }

    /// Read aggregate metadata about the index.
    ///
    /// Runs three read-only queries:
    /// 1. `COUNT(*)` for the row count,
    /// 2. `MIN(timestamp), MAX(timestamp)` for the time range,
    /// 3. `SELECT DISTINCT tag ... ORDER BY tag` for the tag list.
    ///
    /// On an empty database, returns `entries = 0`, both timestamp bounds
    /// as `None`, and an empty `tags` vector — not an error.
    pub fn stats(&self) -> Result<Stats> {
        // COUNT(*) is always non-negative; cast i64 → u64 is well-defined.
        let entries_i64: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM log_entries", [], |row| row.get(0))?;
        let entries = entries_i64 as u64;

        // Aggregates without GROUP BY always yield exactly one row; MIN/MAX
        // on an empty table return (NULL, NULL), which maps cleanly to
        // (None, None) via rusqlite's Option<T> FromSql impl.
        let (min_timestamp, max_timestamp): (Option<String>, Option<String>) =
            self.conn.query_row(
                "SELECT MIN(timestamp), MAX(timestamp) FROM log_entries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

        // SQLite's `ORDER BY tag` (default ascending) places NULLs first,
        // which is exactly the ordering contract advertised on `Stats.tags`.
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT tag FROM log_entries ORDER BY tag")?;
        let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;
        let mut tags: Vec<Option<String>> = Vec::new();
        for row in rows {
            tags.push(row?);
        }

        Ok(Stats {
            entries,
            min_timestamp,
            max_timestamp,
            tags,
        })
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn ensure_parent_dir(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        // Relative filename with no directory component ("index.db").
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|io_err| LogdiveError::io_at(parent, io_err))
}

fn init_schema(conn: &Connection) -> Result<()> {
    // Schema taken verbatim from the project doc's "SQLite schema" section,
    // with `IF NOT EXISTS` added on every statement so open() is idempotent.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS log_entries (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp   TEXT NOT NULL,
            level       TEXT,
            message     TEXT,
            tag         TEXT,
            fields      TEXT,
            raw         TEXT NOT NULL,
            raw_hash    TEXT NOT NULL UNIQUE,
            ingested_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_level      ON log_entries(level);
        CREATE INDEX IF NOT EXISTS idx_tag        ON log_entries(tag);
        CREATE INDEX IF NOT EXISTS idx_timestamp  ON log_entries(timestamp);
        CREATE INDEX IF NOT EXISTS idx_level_norm ON log_entries(lower(level));",
    )?;
    Ok(())
}

fn insert_one_chunk(conn: &mut Connection, entries: &[LogEntry]) -> Result<InsertStats> {
    let tx = conn.transaction()?;
    let mut stats = InsertStats::default();

    {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO log_entries
             (timestamp, level, message, tag, fields, raw, raw_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;

        for entry in entries {
            // NOT NULL enforcement — see module-level docs.
            let Some(ref ts) = entry.timestamp else {
                stats.skipped_no_timestamp += 1;
                continue;
            };

            // Serializing a `Map<String, Value>` via serde_json is infallible:
            // every `Value` variant has a defined JSON representation.
            let fields_json = serde_json::to_string(&entry.fields)
                .expect("serializing serde_json::Map<String, Value> is infallible");
            let raw_hash = blake3::hash(entry.raw.as_bytes()).to_hex().to_string();

            let changes = stmt.execute(params![
                ts,
                entry.level,
                entry.message,
                entry.tag,
                fields_json,
                entry.raw,
                raw_hash,
            ])?;

            if changes == 0 {
                stats.deduplicated += 1;
            } else {
                stats.inserted += 1;
