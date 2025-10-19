//! Cross-format ingestion and deduplication tests.
//!
//! These tests verify that the dedup contract (blake3 hash of the raw line)
//! behaves correctly across all three supported formats and in mixed-format
//! batches. Key invariants:
//!
//! - The same raw line ingested twice is deduplicated regardless of format.
//! - Semantically equivalent content in different formats has different raw
//!   bytes and therefore different hashes — two separate rows result.
//! - Plain-text entries require an explicit timestamp to survive the indexer's
//!   `timestamp NOT NULL` constraint; the test sets one directly.

use logdive_core::{
    Indexer,
    parsers::{json, logfmt, plain},
};

// ---------------------------------------------------------------------------
// JSON dedup
// ---------------------------------------------------------------------------

#[test]
fn same_json_line_ingested_twice_is_deduplicated() {
    let mut idx = Indexer::open_in_memory().unwrap();

    let line = r#"{"timestamp":"2026-04-20T10:00:00Z","level":"info","message":"hello"}"#;
    let entry = json::parse_line(line).expect("fixture must parse");

    idx.insert_batch(std::slice::from_ref(&entry)).unwrap();
    let stats = idx.insert_batch(std::slice::from_ref(&entry)).unwrap();

    assert_eq!(stats.deduplicated, 1, "second insert must be a dedup hit");
    assert_eq!(stats.inserted, 0);

    let count: i64 = idx
        .connection()
        .query_row("SELECT COUNT(*) FROM log_entries", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "table must contain exactly one row");
}

// ---------------------------------------------------------------------------
// Logfmt dedup
// ---------------------------------------------------------------------------
