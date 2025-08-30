//! Concurrent CLI ingest tests.
//!
//! These tests spawn two `logdive ingest` processes against the same database
//! file simultaneously and verify that:
//!
//! - Neither process crashes or corrupts the database (SQLite's file-locking
//!   serializes concurrent writers; one process waits while the other commits).
//! - Deduplication is preserved: ingesting the same file from two processes
//!   results in each unique row being stored exactly once.

use std::process::Command;

use logdive_core::Indexer;

const LOGDIVE: &str = env!("CARGO_BIN_EXE_logdive");

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_json_log(path: &std::path::Path, n: usize) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).expect("create log file");
    for i in 0..n {
        writeln!(
            f,
            r#"{{"timestamp":"2026-04-20T{:02}:{:02}:00Z","level":"info","message":"entry-{i}"}}"#,
            i / 60,
            i % 60,
        )
        .expect("write log line");
    }
}

fn ingest(db: &std::path::Path, log: &std::path::Path) -> std::process::Child {
    Command::new(LOGDIVE)
        .args([
            "--db",
            db.to_str().unwrap(),
