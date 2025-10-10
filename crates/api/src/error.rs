//! HTTP-facing error type for the `logdive-api` server.
//!
//! Classifies errors into three HTTP-shaped buckets:
//!   - [`AppError::BadRequest`] — the client sent something malformed
//!     (missing `q`, unparseable query expression, bad datetime).
//!   - [`AppError::NotFound`] — an explicit miss by an endpoint, reserved
//!     for future endpoints that look up specific records. Not used for
//!     route-level misses; Axum handles unknown-route 404s on its own.
//!   - [`AppError::Internal`] — any failure below the request boundary
//!     (SQLite error, corrupt JSON in the index, I/O failure). These
//!     are logged in full to tracing but shown to the client only as a
//!     generic `"internal server error"` message.
//!
//! The status-code mapping lives in exactly one place: the `From<LogdiveError>`
//! impl. Handlers can therefore use `?` on any `LogdiveError`-returning
//! operation and get correct classification for free.
//!
//! For source types that are *not* `LogdiveError` but whose `LogdiveError`
//! conversion is already defined in core (e.g. [`QueryParseError`] via its
//! `#[from]` variant on `LogdiveError::QueryParse`), this module provides
//! an explicit shim `From` impl. Rust's `?` operator only performs a single
//! `From` conversion, so `Result<T, QueryParseError> -> Result<T, AppError>`
//! needs its own direct impl rather than going through `LogdiveError`
//! implicitly.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use logdive_core::{LogdiveError, QueryParseError};

/// HTTP error surface used across all handlers.
#[derive(Debug)]
pub enum AppError {
    /// Client sent a malformed request. Message is user-facing.
    BadRequest(String),

    /// A specific resource was not present. Message is user-facing.
    ///
    /// Reserved for future endpoints that look up by id. The current
    /// `GET /query` and `GET /stats` never emit this; returning zero
    /// matches from a query is a `200 OK` with an empty body, not a 404.
    #[allow(dead_code)]
    NotFound(String),

    /// Unexpected internal failure. The underlying `LogdiveError` is
    /// kept for operator-side logging and is never exposed to the
    /// client.
    Internal(LogdiveError),
}

impl AppError {
    /// Convenience constructor for 400 responses with a `Display` source.
    pub fn bad_request<M: std::fmt::Display>(msg: M) -> Self {
        Self::BadRequest(msg.to_string())
    }
}

/// Map `LogdiveError` variants to appropriate HTTP error classes.
///
/// This is the single source of truth for classification — handlers simply
/// use `?` and rely on this impl to do the right thing. Anything that
/// looks like "the user sent something bad" becomes `BadRequest`;
/// anything else becomes `Internal`.
impl From<LogdiveError> for AppError {
    fn from(err: LogdiveError) -> Self {
        match &err {
            LogdiveError::QueryParse(_)
            | LogdiveError::InvalidDatetime { .. }
            | LogdiveError::UnsafeFieldName(_) => AppError::BadRequest(err.to_string()),
            _ => AppError::Internal(err),
        }
    }
}

/// Explicit bridge from `QueryParseError` to `AppError`.
///
/// `parse_query` in core returns `Result<_, QueryParseError>` directly,
/// not wrapped in `LogdiveError`. Rust's `?` only performs a single
/// conversion via `From`, so even though `LogdiveError: From<QueryParseError>`
/// is defined in core, callers using `?` on `parse_query(...)?` from an
/// `AppError`-returning function need a direct impl. We delegate to the
/// `LogdiveError` path so classification stays in one place.
impl From<QueryParseError> for AppError {
    fn from(err: QueryParseError) -> Self {
        AppError::from(LogdiveError::from(err))
    }
}

/// JSON body shape returned for every error response.
///
/// Private to this module — handlers never construct one directly.
#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
