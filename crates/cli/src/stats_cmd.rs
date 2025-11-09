//! `stats` subcommand: report aggregate metadata about the index.
//!
//! Output is human-readable only, per the decisions log and the milestone 7
//! design notes. JSON output for stats lives in the HTTP API (milestone 8),
//! which has a natural serialization target; mirroring it in the CLI would
//! duplicate surface area without clear benefit.
//!
//! Missing-database policy: if the configured index path does not exist on
//! disk, `stats` reports an error and exits non-zero rather than auto-
//! creating an empty database. Creating a DB as a side effect of a read
//! action would hide typos in the `--db` flag behind a misleading
//! "zero entries" readout.

use std::path::Path;

use logdive_core::{Indexer, LogdiveError, Result, Stats};

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

/// Arguments for the `stats` subcommand.
///
/// Empty today — kept as a named type so the subcommand dispatch in `main`
/// is uniform across subcommands, and so milestone 9 can add flags (e.g.
/// `--no-tags`) without restructuring the Clap definition.
#[derive(clap::Args, Debug)]
pub struct StatsArgs {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run `logdive stats` against the index at `db`.
///
/// `db` is resolved by the caller (honoring `--db` or the default
/// `~/.logdive/index.db`). The function checks existence, opens the index,
/// reads [`Stats`], pairs it with on-disk file size, and prints the
/// formatted report to stdout.
pub fn run_stats(db: &Path, _args: StatsArgs) -> Result<()> {
    if !db.exists() {
        return Err(LogdiveError::io_at(
            db,
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "no index found at {}; run `logdive ingest` first",
                    db.display()
                ),
            ),
        ));
    }

    let indexer = Indexer::open(db)?;
    let stats = indexer.stats()?;
    let size_bytes = std::fs::metadata(db)
        .map_err(|e| LogdiveError::io_at(db, e))?
        .len();

    let out = format_stats(db, &stats, size_bytes);
    println!("{out}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Formatting (pure, testable)
// ---------------------------------------------------------------------------

/// Format a `Stats` report as a multi-line human-readable string.
///
/// No trailing newline — `println!` adds one.
pub fn format_stats(db: &Path, stats: &Stats, size_bytes: u64) -> String {
    let mut s = String::new();
    s.push_str(&format!("logdive index: {}\n", db.display()));

    s.push_str(&format!(
        "  Entries:       {}\n",
        with_thousands_separator(stats.entries)
    ));

    s.push_str(&format!("  Time range:    {}\n", format_time_range(stats)));

    s.push_str(&format!("  Tags:          {}\n", format_tags(&stats.tags)));

    // Size is the last line — no trailing newline so the caller's println!
    // terminates cleanly.
    s.push_str(&format!("  DB size:       {}", format_size(size_bytes)));
    s
}

fn format_time_range(stats: &Stats) -> String {
    match (
        stats.min_timestamp.as_deref(),
        stats.max_timestamp.as_deref(),
    ) {
        (Some(min), Some(max)) => format!("{min} → {max}"),
        _ => "(empty)".to_string(),
    }
}

/// Render the tags list per the Q9 display contract:
///
///   1. Non-null tags first, alphabetically ascending.
///   2. `(untagged)` appended at the end if any rows carry NULL tag.
///
/// `Indexer::stats()` returns `None` at the head of the vector (SQLite
/// NULLs-first ordering); here we reshuffle to end-of-list for display.
fn format_tags(tags: &[Option<String>]) -> String {
    if tags.is_empty() {
        return "(none)".to_string();
    }

    let mut named: Vec<&str> = Vec::with_capacity(tags.len());
    let mut has_untagged = false;
    for t in tags {
        match t {
            Some(s) => named.push(s.as_str()),
