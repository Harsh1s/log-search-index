//! End-to-end integration tests for the HTTP API.
//!
//! These tests build a real [`Router`] via [`logdive_api::router::build_router`]
//! against a tempfile-backed SQLite database, then exercise it via
//! [`tower::ServiceExt::oneshot`] — which bypasses the network stack while
//! still running the full extractor → handler → `IntoResponse` pipeline.
//!
//! Coverage spans the two endpoints' happy paths, their user-fault error
//! paths (400), and the default unknown-route behavior (404). Server-fault
//! paths (500) are exercised by the unit tests inside `error.rs`; surfacing
//! them through a live request-response cycle would require deliberately
//! corrupting the DB mid-flight, which adds noise without meaningful
//! additional coverage.

use std::path::PathBuf;

use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt; // for `.oneshot`

use logdive_api::{router::build_router, state::AppState};
use logdive_core::{Indexer, LogEntry};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Create an empty, schema-initialized index in a fresh temp directory.
///
/// Returns the `TempDir` (must be kept alive for the lifetime of the test
/// to prevent early cleanup) and the DB path within it.
fn empty_db() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("index.db");
    // Touch the DB file and create the schema via the core writable opener.
    let _ = Indexer::open(&db).expect("init empty db");
    (dir, db)
}

/// Create a populated index with three entries spanning two tags and two
/// levels, suitable for exercising /query and /stats against real data.
fn populated_db() -> (TempDir, PathBuf) {
    let (dir, db) = empty_db();
    let mut idx = Indexer::open(&db).expect("reopen for populate");
    idx.insert_batch(&[
        entry(
            "2026-04-20T10:00:00Z",
            "error",
            "payment failed",
            Some("api"),
        ),
        entry(
            "2026-04-20T11:00:00Z",
            "info",
            "health check ok",
            Some("api"),
        ),
        entry("2026-04-20T12:00:00Z", "error", "database timeout", None),
    ])
    .expect("insert fixtures");
    (dir, db)
}

fn entry(ts: &str, level: &str, message: &str, tag: Option<&str>) -> LogEntry {
    let tag_part = tag.map(|t| format!(r#","tag":"{t}""#)).unwrap_or_default();
    let raw =
        format!(r#"{{"timestamp":"{ts}","level":"{level}","message":"{message}"{tag_part}}}"#);
    let mut e = LogEntry::new(raw);
    e.timestamp = Some(ts.to_string());
    e.level = Some(level.to_string());
    e.message = Some(message.to_string());
    e.tag = tag.map(|t| t.to_string());
    e
}

fn app(db: PathBuf) -> axum::Router {
    build_router(AppState::new(db), vec![])
}

async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    resp.into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes()
        .to_vec()
}

async fn body_text(resp: axum::response::Response) -> String {
    String::from_utf8(body_bytes(resp).await).expect("utf-8 body")
}

async fn body_json(resp: axum::response::Response) -> Value {
    let text = body_text(resp).await;
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("body is not JSON: {e}; body=`{text}`"))
}

/// Parse an NDJSON body into a `Vec<Value>`. Empty bodies yield an empty vec.
fn parse_ndjson(text: &str) -> Vec<Value> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("each NDJSON line is valid JSON"))
        .collect()
}

// ---------------------------------------------------------------------------
// GET /stats
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stats_on_populated_db_returns_expected_shape() {
    let (_dir, db) = populated_db();
    let router = app(db.clone());

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(content_type.starts_with("application/json"));

    let v = body_json(resp).await;
    assert_eq!(v["entries"], 3);
    assert_eq!(v["min_timestamp"], "2026-04-20T10:00:00Z");
    assert_eq!(v["max_timestamp"], "2026-04-20T12:00:00Z");

    // Tags: null (untagged row) first, then "api". Clients get the raw
    // core ordering; no "(untagged)" rewriting at this layer.
    let tags = v["tags"].as_array().expect("tags is array");
    assert_eq!(tags.len(), 2);
    assert!(tags[0].is_null());
    assert_eq!(tags[1], "api");

    assert_eq!(v["db_path"], db.display().to_string());
    assert!(v["db_size_bytes"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn stats_on_empty_db_returns_zeroed_values() {
    let (_dir, db) = empty_db();
    let router = app(db);

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries"], 0);
    assert!(v["min_timestamp"].is_null());
    assert!(v["max_timestamp"].is_null());
    assert_eq!(v["tags"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// GET /query
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_with_matching_expression_returns_ndjson() {
    let (_dir, db) = populated_db();
    let router = app(db);

    let resp = router
        .oneshot(
            Request::builder()
                // level=error matches two of the three fixture rows.
                .uri("/query?q=level%3Derror")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(ct, "application/x-ndjson");

    let text = body_text(resp).await;
    let rows = parse_ndjson(&text);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r["level"] == "error"));
}

#[tokio::test]
