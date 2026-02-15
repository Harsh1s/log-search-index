//! `logdive` CLI binary.
//!
//! Four subcommands: `ingest`, `query`, `stats`, `prune`. A global `--db`
//! flag selects the index path; all subcommands respect it. The flag also
//! reads the `LOGDIVE_DB` environment variable as a fallback, matching the
//! `logdive-api` binary — the command-line value wins when both are set.
//!
//! # Changes in v0.2.0
//!
//! - `--format json|logfmt|plain` on `ingest` (M2: multi-format ingestion).
//! - `--timestamp-now` on `ingest` (M2: universal fallback timestamp).
//! - `--follow` on `ingest` with `--file` (M3: file tailing, Unix only).
//! - `prune` subcommand (M4: time-based retention).
//! - `LOGDIVE_DB` environment-variable fallback for `--db` (M4).
//!
//! # Changes in v0.3.0
//!
//! - `--output pretty|json` on `query` replaces `--format` (B3: unambiguous
//!   flag name — `--format` on `ingest` is the input format, so reusing it
//!   on `query` for the output format was confusing).
//! - `--limit` and `--offset` on `query` for result-set pagination (B2).

mod prune_cmd;
mod render;
mod stats_cmd;

use std::io::{self, BufRead, IsTerminal};
use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use logdive_core::{
    Indexer, InsertStats, LogEntry, LogFormat, LogdiveError, QueryOptions, Result, db_path,
    execute, parse_line, parse_query,
};
use prune_cmd::{PruneArgs, run_prune};
use render::{OutputFormat, render};
use stats_cmd::{StatsArgs, run_stats};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Fast, self-hosted query engine for structured JSON logs.
#[derive(Parser, Debug)]
#[command(name = "logdive", version, about, long_about = None)]
struct Cli {
    /// Path to the index database. Defaults to ~/.logdive/index.db.
    ///
    /// Applies to all subcommands. Can also be set via the `LOGDIVE_DB`
    /// environment variable; the command-line flag takes precedence when
    /// both are provided.
    #[arg(long, global = true, value_name = "PATH", env = "LOGDIVE_DB")]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Ingest structured log lines from a file or stdin into the index.
    Ingest(IngestArgs),
    /// Query the index and render matching log entries.
    Query(QueryArgs),
    /// Report aggregate metadata about the index.
    Stats(StatsArgs),
    /// Delete entries older than a cutoff, then VACUUM the database.
    Prune(PruneArgs),
}

/// Arguments for the `ingest` subcommand.
#[derive(clap::Args, Debug)]
pub struct IngestArgs {
    /// Read from this file instead of stdin.
    #[arg(long, short = 'f', value_name = "PATH")]
    file: Option<PathBuf>,

    /// Attach a tag to every ingested entry that lacks a `tag` field.
    #[arg(long, short = 't', value_name = "TAG")]
    tag: Option<String>,

    /// Input format of the log lines.
    ///
    /// `json` (default) expects newline-delimited JSON objects.
    /// `logfmt` expects `key=value` pairs.
    /// `plain` treats each line as an unstructured message.
    #[arg(
        long,
        value_name = "FORMAT",
        default_value = "json",
        value_parser = parse_log_format
    )]
    format: LogFormat,

    /// Stamp the current ingestion time on entries that have no timestamp.
    ///
    /// Without this flag, timestamp-less entries are silently skipped
    /// (no-fabrication policy). Most useful with `--format plain`.
    #[arg(long)]
    timestamp_now: bool,

    /// Watch the file for newly appended lines and ingest them continuously.
    ///
    /// Requires `--file`. Stdin already streams until EOF; `--follow` is
    /// not needed and is rejected with an actionable error message.
    ///
    /// Unix only. Exits cleanly on Ctrl-C.
    #[arg(long, requires = "file")]
    follow: bool,

    /// Exit the follow loop after this many filesystem events.
    ///
    /// Hidden flag for deterministic testing of the watch loop; not
    /// intended for end-user use.
    #[arg(long, value_name = "N", hide = true)]
    max_events: Option<usize>,
}

/// Arguments for the `query` subcommand.
#[derive(clap::Args, Debug)]
struct QueryArgs {
    /// Query expression (e.g. `level=error AND service=payments last 2h`).
    query: String,

    /// Output format. `pretty` (default) is human-readable and optionally
    /// colored; `json` emits newline-delimited JSON suitable for piping
    /// into `jq` or other tools.
    #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
    output: OutputFormat,

    /// Maximum number of results to return. 0 means unlimited.
    #[arg(long, default_value_t = 1000)]
    limit: usize,

    /// Number of results to skip from the front of the ordered result set.
    /// Use with `--limit` to page through large result sets.
    /// 0 (default) starts from the first result.
    #[arg(long, default_value_t = 0)]
    offset: usize,
}

// ---------------------------------------------------------------------------
// Clap value parser for LogFormat (keeps the clap dependency out of core)
// ---------------------------------------------------------------------------
