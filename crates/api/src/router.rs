//! Axum router construction.
//!
//! Extracted from `main.rs` so integration tests can build the same router
//! the binary uses without duplicating route definitions. The router is
//! pure data — no I/O happens here; `AppState` carries the configuration
//! and all I/O is deferred into the handler layer.
//!
//! [`build_router`] accepts a pre-parsed list of allowed CORS origins.
//! Parsing, validation, and the wildcard `*` check happen in `main.rs`,
//! keeping this module free of clap and environment-variable concerns.

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::handlers::{query_handler, stats_handler, version_handler};
use crate::state::AppState;

/// Build the application router with all endpoints wired up.
///
/// `cors_origins` controls the CORS policy applied to every route:
///
/// - `[]` (empty) — CORS disabled; no `Access-Control-Allow-Origin` header
///   is ever added. This is the default and appropriate for local or
///   server-side-only consumers.
/// - `[HeaderValue::from_static("*")]` — wildcard; any origin is reflected.
///   The single-element invariant is enforced by the caller in `main.rs`.
/// - Any other non-empty list — exactly those origins are allowed; the
///   matching origin is reflected back in the response header.
///
/// The returned router is ready for `axum::serve` in the binary or
/// `tower::ServiceExt::oneshot` in tests.
pub fn build_router(state: AppState, cors_origins: Vec<HeaderValue>) -> Router {
    let router = Router::new()
        .route("/query", get(query_handler))
        .route("/stats", get(stats_handler))
        .route("/version", get(version_handler))
        .with_state(state);

    match build_cors_layer(cors_origins) {
        Some(cors) => router.layer(cors),
        None => router,
    }
}

/// Construct a [`CorsLayer`] from the parsed origin list, or return `None`
/// when CORS should be disabled entirely (empty list).
///
/// Kept private and separate from [`build_router`] so the CORS policy
/// logic is testable without constructing a full router.
///
