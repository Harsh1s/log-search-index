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
