//! `logdive-api` binary entry point.
//!
//! Reads configuration from command-line flags (with environment-variable
//! fallbacks), ensures the index exists (creating an empty one on first
//! run if needed), then hands the built router to `axum::serve` with
//! graceful-shutdown wiring.
//!
//! The actual HTTP surface lives in the `logdive_api` library half of
//! this crate — see `lib.rs` for the module map.

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::http::HeaderValue;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use logdive_api::router::build_router;
use logdive_api::state::AppState;
use logdive_core::{Indexer, LogdiveError, Result, db_path};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "logdive-api",
    version,
    about = "Read-only HTTP API server for a logdive index",
    long_about = None,
)]
struct Cli {
    /// Path to the index database. Defaults to ~/.logdive/index.db.
    ///
    /// Can also be set via the `LOGDIVE_DB` environment variable; the
    /// command-line value takes precedence when both are provided.
    #[arg(long, value_name = "PATH", env = "LOGDIVE_DB")]
    db: Option<PathBuf>,

    /// Port to listen on.
    ///
    /// Can also be set via `LOGDIVE_API_PORT`. Default 4000.
    #[arg(long, default_value_t = 4000, env = "LOGDIVE_API_PORT")]
    port: u16,

    /// Host/IP to bind to.
    ///
    /// Defaults to `127.0.0.1` — loopback only. Set explicitly to
    /// `0.0.0.0` (or a specific non-loopback address) to expose the
    /// server beyond localhost. Can also be set via `LOGDIVE_API_HOST`.
    #[arg(long, default_value = "127.0.0.1", env = "LOGDIVE_API_HOST")]
    host: String,

    /// Comma-separated list of origins allowed to make cross-origin requests.
    ///
    /// Use `*` as the sole value to allow any origin. Omit the flag (or
    /// leave it empty) to disable CORS entirely — same-origin requests
    /// are always served regardless of this setting. Invalid values cause
    /// a startup error.
    ///
    /// Examples:
    ///   --cors-origins '*'
    ///   --cors-origins 'https://app.example.com,https://staging.example.com'
    ///
    /// Can also be set via `LOGDIVE_API_CORS_ORIGINS`; the command-line
    /// value takes precedence when both are provided.
    #[arg(long, value_name = "ORIGINS", env = "LOGDIVE_API_CORS_ORIGINS")]
    cors_origins: Option<String>,

    /// Run a TCP connectivity check against the server's own port and exit.
    ///
    /// Exits 0 if the port is reachable, 1 otherwise. Never starts the HTTP
    /// server. Intended for Docker `HEALTHCHECK` — a pure-stdlib alternative
    /// to `curl` that works in distroless images with no shell or tools.
    ///
    /// Example:
    ///   HEALTHCHECK CMD ["/usr/local/bin/logdive-api", "--health-check"]
    #[arg(long)]
    health_check: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();

    if cli.health_check {
        run_health_check(cli.port);
    }

    // Validate CORS config before touching the filesystem — a bad origin
    // string is a configuration error that should surface immediately, not
    // after the DB existence check.
    let cors_origins = parse_cors_origins(cli.cors_origins).unwrap_or_else(|msg| {
        eprintln!("error: invalid --cors-origins: {msg}");
        std::process::exit(1);
    });

    // Resolve the DB path the same way the CLI does, so env/default
    // behavior is consistent across the two surfaces.
    let db = db_path(cli.db.as_deref());

    // Ensure the index exists. On first run (e.g. a fresh Docker volume or a
    // new installation) the file is absent — in that case we create an empty
    // index with the correct schema so the server starts cleanly and returns
    // zero results until logs are ingested.
    //
    // This preserves the "fail fast" property for genuinely bad paths:
    // a wrong directory or permission error surfaces here as a startup
    // failure rather than as a flurry of 500s per request.
    ensure_index_exists(&db)?;

    // Build state and router.
    let state = AppState::new(db.clone());
    let app = build_router(state, cors_origins.clone());

    // Bind. Parsing the host string through `format!` + `parse` keeps the
    // error path uniform: any malformed host goes through `io_at`.
    let addr: SocketAddr =
        format!("{}:{}", cli.host, cli.port)
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                LogdiveError::io_at(
                    &db,
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("invalid host:port `{}:{}`: {e}", cli.host, cli.port),
                    ),
                )
            })?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| LogdiveError::io_at(&db, e))?;

    let bound = listener
        .local_addr()
        .map_err(|e| LogdiveError::io_at(&db, e))?;

    let cors_desc = cors_summary(&cors_origins);
    tracing::info!(
        %bound,
        index = %db.display(),
        cors = %cors_desc,
        "logdive-api listening",
    );
    eprintln!(
        "logdive-api listening on http://{bound} (index: {})",
        db.display()
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| LogdiveError::io_at(&db, e))?;

    tracing::info!("logdive-api shutdown complete");
    Ok(())
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

/// TCP-connect to the server's own port and exit the process.
///
/// Exits 0 when the connection succeeds, 1 when it fails. No HTTP request is
