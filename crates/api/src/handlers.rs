//! Axum handlers for the three public endpoints.
//!
//! All handlers are async, but every one that touches the database
//! delegates SQLite work to [`AppState::with_connection`], which runs on
//! Tokio's blocking-task pool. The handlers themselves do parsing,
//! parameter validation, and response shaping — nothing else.
//!
//! `GET /version` is the exception: it returns compile-time constants only
//! and never opens the database.

use axum::{
    Json,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use logdive_core::{LogEntry, LogFormat, QueryOptions, Stats, execute, parse_query};

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /query
// ---------------------------------------------------------------------------

/// Query-string parameters accepted by `GET /query`.
///
/// `q` is modeled as `Option<String>` rather than a required field so the
/// "missing `q`" error message comes from our own `AppError::bad_request`
/// path rather than from Axum's generic extractor rejection.
///
/// `offset` is optional; absent or `0` both mean "start from the first result".
/// Mirrors the CLI's `--offset` behaviour so both surfaces page identically.
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Default cap on result set size when `limit` is not supplied. Mirrors
/// the CLI's `--limit` default so the two surfaces behave identically.
const DEFAULT_LIMIT: usize = 1000;

/// `GET /query?q=<expr>&limit=<n>&offset=<m>`
///
/// Returns matching log entries as newline-delimited JSON, one entry per
/// line. A missing `limit` defaults to 1000; `limit=0` means unlimited.
/// A missing or zero `offset` starts from the first result.
pub async fn query_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Response, AppError> {
    // Validate: `q` is required and non-empty (post-trim).
    let query_str = params
        .q
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("missing or empty `q` parameter"))?;

    // Apply the same "0 = unlimited" and "0 = absent" rules as the CLI.
    let limit = match params.limit.unwrap_or(DEFAULT_LIMIT) {
        0 => None,
        n => Some(n),
    };
    let offset = match params.offset.unwrap_or(0) {
        0 => None,
        n => Some(n),
    };

    // Parse the query up-front so parse errors are classified by the
    // `From<LogdiveError>` impl before we touch the DB.
    let ast = parse_query(&query_str)?;

    tracing::debug!(query = %query_str, ?limit, ?offset, "executing query over HTTP");

    let rows: Vec<LogEntry> = state
        .with_connection(move |indexer| {
            execute(&ast, indexer.connection(), QueryOptions { limit, offset })
        })
        .await?;

    tracing::debug!(
        result_count = rows.len(),
        "query returned results over HTTP"
    );

    Ok(build_ndjson_response(&rows))
}

/// Serialize a slice of entries into a newline-delimited JSON body and
/// wrap it in a response with the correct `Content-Type`.
///
/// Empty result sets produce an empty body (zero bytes, zero lines),
/// which is valid NDJSON and pipeline-friendly. Status is 200 OK in
/// both the empty and non-empty cases — "no matches" is a successful
/// query, not a not-found.
fn build_ndjson_response(rows: &[LogEntry]) -> Response {
    let mut body = String::with_capacity(rows.len() * 256);
    for row in rows {
        body.push_str(&entry_to_json_string(row));
        body.push('\n');
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        body,
    )
        .into_response()
}

/// Render a single `LogEntry` as a JSON object string.
///
/// The shape matches the CLI's `--format json` output so clients can treat
/// both surfaces interchangeably: `timestamp`, `level`, `message`, `tag`,
/// `fields`, `raw`. `tag` is `null` when absent; `fields` is always an
/// object.
fn entry_to_json_string(entry: &LogEntry) -> String {
    serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string())
}

// ---------------------------------------------------------------------------
// GET /stats
// ---------------------------------------------------------------------------

/// Wire shape for `GET /stats` responses.
///
/// Intentionally decoupled from `logdive_core::Stats` so the library stays
/// serde-agnostic on its public output types. Renaming core fields is
/// then a non-breaking change for HTTP clients; conversely, the HTTP
/// shape can evolve without touching core.
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub entries: u64,
    pub min_timestamp: Option<String>,
    pub max_timestamp: Option<String>,
    /// Distinct tag values. `None` (untagged) appears first when present,
    /// then non-null tags in ascending alphabetical order — identical to
    /// the `Stats.tags` contract from core. Clients presenting this to
    /// humans can reshuffle (e.g. put `null` last, label it "(untagged)")
    /// the way the CLI does.
    pub tags: Vec<Option<String>>,
    pub db_size_bytes: u64,
    pub db_path: String,
}

impl StatsResponse {
    fn from_core(stats: Stats, db_path: String, db_size_bytes: u64) -> Self {
        Self {
            entries: stats.entries,
            min_timestamp: stats.min_timestamp,
            max_timestamp: stats.max_timestamp,
            tags: stats.tags,
            db_size_bytes,
            db_path,
        }
    }
}

/// `GET /stats`
///
/// Returns aggregate metadata about the index as a single JSON object.
pub async fn stats_handler(State(state): State<AppState>) -> Result<Json<StatsResponse>, AppError> {
    let db_path_for_response = state.db_path.display().to_string();
    let db_path_for_closure = state.db_path.clone();

    let (stats, size_bytes) = state
        .with_connection(move |indexer| {
            // Run both queries under the same blocking task so we don't
            // round-trip back to the async runtime between them.
            let stats = indexer.stats()?;
            let size_bytes = std::fs::metadata(&db_path_for_closure)
                .map(|m| m.len())
                .map_err(|e| logdive_core::LogdiveError::io_at(&db_path_for_closure, e))?;
            Ok::<_, logdive_core::LogdiveError>((stats, size_bytes))
        })
        .await?;

    let response = StatsResponse::from_core(stats, db_path_for_response, size_bytes);
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// GET /version
// ---------------------------------------------------------------------------

/// Wire shape for `GET /version` responses.
///
/// All fields are compile-time constants — no database access is required.
/// The primary use case is client-side feature detection: a UI or script
/// calls `/version` on startup to discover which formats and endpoints the
/// running server supports before making assumptions about its capabilities.
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    /// Semver version of the running `logdive-api` binary, injected from
    /// `CARGO_PKG_VERSION` at compile time.
    pub version: &'static str,
    /// Ingest formats the binary was compiled with, sourced from
    /// [`LogFormat::ALL`]. Stays in sync with core without manual
    /// maintenance — adding a new format to core propagates here for free.
    pub formats: Vec<&'static str>,
    /// API endpoint names available on this server, sorted alphabetically.
    pub capabilities: Vec<&'static str>,
}

/// `GET /version`
///
/// Returns a JSON object describing the running server's version and
/// capabilities. Built entirely from compile-time constants; never touches
/// the database. Always returns `200 OK`.
pub async fn version_handler() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        formats: LogFormat::ALL.iter().map(|f| f.name()).collect(),
        capabilities: vec!["query", "stats", "version"],
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample_entry() -> LogEntry {
        let raw = r#"{"timestamp":"2026-04-22T14:03:21Z","level":"error","message":"boom"}"#;
        let mut e = LogEntry::new(raw.to_string());
        e.timestamp = Some("2026-04-22T14:03:21Z".to_string());
        e.level = Some("error".to_string());
        e.message = Some("boom".to_string());
        e
    }

    #[test]
    fn entry_to_json_string_is_valid_json_with_expected_shape() {
        let e = sample_entry();
