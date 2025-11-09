//! `prune` subcommand: time-based retention for the index.
//!
//! Deletes entries older than a caller-specified cutoff, then `VACUUM`s the
//! database to reclaim disk space. The cutoff is given one of two ways,
//! exactly one of which is required (enforced by a clap `ArgGroup`):
//!
//! - `--older-than <DURATION>` — relative. `30d`, `2h`, `45m`. The cutoff is
//!   computed as "now minus DURATION", so `--older-than 30d` keeps the last
//!   30 days and deletes everything before that.
//! - `--before <DATETIME>` — absolute. Accepts the same three datetime
//!   shapes the query language's `since` clause accepts: RFC3339, a naive
//!   `YYYY-MM-DD HH:MM:SS` (interpreted as UTC), or a bare `YYYY-MM-DD`
//!   date (interpreted as UTC midnight).
//!
//! Duration resolution lives here rather than in `logdive-core` because it
//! is a CLI-surface concern: core's [`Indexer::prune`] takes an already
//! resolved RFC3339 cutoff string and stays a clean library boundary. There
//! is minor overlap with the executor's private datetime parsing, but
//! widening core's *published* public API for a CLI-only concern is the
//! worse trade.
//!
//! # Safety
//!
//! `prune` is destructive. By default it prints how many entries the cutoff
//! would delete and asks for an interactive `[y/N]` confirmation; anything
//! other than `y` / `yes` aborts without touching the index. The `--yes`
//! flag bypasses the prompt for cron jobs and scripts. If the cutoff would
//! delete nothing, `prune` says so and exits without prompting.
//!
//! Missing-database policy mirrors `stats`: if the configured index path
//! does not exist, `prune` errors rather than auto-creating an empty
//! database — pruning a database that isn't there is a user mistake worth
//! surfacing, not silently succeeding against an empty index.

use std::io::{self, Write};
use std::path::Path;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};

use logdive_core::{Indexer, LogdiveError, Result};

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

/// Arguments for the `prune` subcommand.
///
/// The `prune_cutoff` group makes `--older-than` and `--before` mutually
/// exclusive and requires exactly one of them: clap rejects both supplying
/// neither and supplying both before `run_prune` is ever called.
#[derive(clap::Args, Debug)]
#[command(group(
    clap::ArgGroup::new("prune_cutoff")
        .required(true)
        .args(["older_than", "before"])
))]
pub struct PruneArgs {
    /// Delete entries older than this duration relative to now.
    ///
    /// A whole number followed by a unit: `m` (minutes), `h` (hours), or
    /// `d` (days). Examples: `30d`, `12h`, `90m`.
    #[arg(long, value_name = "DURATION")]
    older_than: Option<String>,

    /// Delete entries with a timestamp strictly before this datetime.
    ///
    /// Accepts RFC3339 (`2026-01-01T00:00:00Z`), a naive datetime
    /// (`2026-01-01 00:00:00`, interpreted as UTC), or a bare date
    /// (`2026-01-01`, interpreted as UTC midnight).
    #[arg(long, value_name = "DATETIME")]
    before: Option<String>,

    /// Skip the interactive confirmation prompt.
    ///
    /// Intended for scripts and scheduled jobs. Without this flag, `prune`
    /// shows how many entries would be deleted and waits for a `y` / `yes`
    /// answer on stdin.
    #[arg(long)]
    yes: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run `logdive prune` against the index at `db`.
///
/// Resolves the cutoff from whichever flag was supplied, counts the doomed
/// rows for the confirmation prompt, deletes them via [`Indexer::prune`]
/// (which also `VACUUM`s), and prints a summary.
pub fn run_prune(db: &Path, args: PruneArgs) -> Result<()> {
    // Fail fast on a missing index, matching `stats` — don't auto-create.
    if !db.exists() {
        return Err(LogdiveError::io_at(
            db,
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no index found at {}; nothing to prune", db.display()),
            ),
        ));
    }

    // Resolve the cutoff. clap's ArgGroup guarantees exactly one flag is
    // set, but `resolve_cutoff` handles the other shapes defensively rather
    // than panicking. A bad duration / datetime string surfaces here as an
    // `InvalidInput` I/O error so the CLI's top-level handler prints it.
    let cutoff = resolve_cutoff(&args)
        .map_err(|msg| LogdiveError::IoBare(io::Error::new(io::ErrorKind::InvalidInput, msg)))?;
    let cutoff_rfc3339 = cutoff.to_rfc3339();

    let mut indexer = Indexer::open(db)?;

    // Count what the cutoff would delete: this both makes the confirmation
    // prompt informative and lets us skip prompting entirely when there is
    // nothing to do. Strict `<` matches `Indexer::prune`'s own comparison.
    let doomed: i64 = indexer.connection().query_row(
        "SELECT COUNT(*) FROM log_entries WHERE timestamp < ?1",
        [&cutoff_rfc3339],
        |row| row.get(0),
    )?;

    if doomed == 0 {
        println!("Nothing to prune — no entries older than {cutoff_rfc3339}.");
        return Ok(());
    }

    if !args.yes && !confirm(doomed, &cutoff_rfc3339)? {
        println!("Aborted. No entries deleted.");
        return Ok(());
    }

    let stats = indexer.prune(&cutoff_rfc3339)?;
    println!(
        "Pruned {} {} older than {cutoff_rfc3339}.",
        stats.deleted,
        plural_entries(stats.deleted)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Cutoff resolution (pure, testable)
// ---------------------------------------------------------------------------

/// Resolve the prune cutoff from whichever flag clap accepted.
///
/// Returns `Err` with a user-facing message on a malformed duration or
/// datetime, or on the both-set / neither-set states (which clap's
/// `ArgGroup` already rules out, handled here only so a future grouping
/// change can't turn into a panic).
fn resolve_cutoff(args: &PruneArgs) -> std::result::Result<DateTime<Utc>, String> {
    match (&args.older_than, &args.before) {
        (Some(spec), None) => resolve_older_than(spec, Utc::now()),
        (None, Some(spec)) => resolve_before(spec),
        (Some(_), Some(_)) => Err("--older-than and --before are mutually exclusive".to_string()),
        (None, None) => Err("one of --older-than or --before is required".to_string()),
    }
}

/// Resolve a relative `--older-than` duration spec against a given `now`.
///
/// `now` is a parameter rather than read internally so the unit tests can
