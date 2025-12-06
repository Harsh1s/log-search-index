//! Shared application state for the HTTP server.
//!
//! `AppState` carries the configured database path and offers a uniform
//! helper, [`AppState::with_connection`], for running blocking SQLite work
//! on Tokio's blocking-task pool. Every handler that touches the database
//! routes through this helper so that:
//!   1. The rusqlite dependency stays contained to one module,
//!   2. No handler accidentally blocks the async runtime,
//!   3. Each request gets a fresh read-only connection — matching the
//!      milestone 8 design decision on connection strategy.
//!
//! The read-only connection is opened via [`logdive_core::Indexer::
//! open_read_only`], which enforces SQLite-level `SQLITE_OPEN_READ_ONLY`
//! semantics and fails fast if the DB file is missing (as opposed to
//! creating it, the way `Indexer::open` does).

use std::path::PathBuf;

use logdive_core::{Indexer, LogdiveError, Result};

/// State shared across every HTTP handler.
///
/// Cheap to clone: a single `PathBuf` per instance. Axum requires the
/// state type to be `Clone` so each request handler can get its own
/// owned copy via the `State` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Absolute or resolved path to the logdive index database.
    ///
    /// Opened read-only per request; never modified by the server.
    pub db_path: PathBuf,
}

impl AppState {
    /// Construct a new `AppState` for the given database path.
    ///
    /// Does not perform any I/O — existence/readability of the file is
    /// checked at startup in `main`, and each request re-opens the file
    /// read-only via [`AppState::with_connection`].
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Run `f` on Tokio's blocking-task pool with a fresh read-only
    /// [`Indexer`] over the configured database.
    ///
    /// Propagates the closure's result through as-is. Any error from
    /// opening the database, or a blocking-task join failure (which
    /// happens only if the closure itself panics), is folded into
    /// [`LogdiveError`] — handlers map this to an HTTP `AppError` at the
    /// response boundary.
    ///
    /// # Why `spawn_blocking`
    ///
    /// `rusqlite` calls are synchronous and can do real work (microseconds
