//! Ingestion throughput benchmarks.
//!
//! Measures the full ingestion path: parse each JSON line, construct a
//! `LogEntry`, blake3-hash the raw line, and insert a batch into a fresh
//! on-disk SQLite index.
//!
//! We report throughput in elements (lines) per second via
//! `Throughput::Elements`. The primary number users want from these
//! benchmarks is "how many lines per second can logdive index?" and
//! criterion's throughput reporting surfaces that directly alongside
//! the time-per-batch measurement.
//!
//! Each sample starts from an empty temp-dir-backed database. We do not
//! reuse the database across iterations because `INSERT OR IGNORE` would
//! turn subsequent samples into no-ops (measuring hash-check speed, not
//! ingestion).

use std::path::PathBuf;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;

use logdive_core::{Indexer, LogFormat, parse_line};

/// Generate `n` synthetic JSON log lines. Deterministic — same `n`
/// produces the same output across runs and machines.
///
/// The mix of fields (level, message, service, user_id, duration_ms) is
/// chosen to exercise both known-column lifts and the JSON blob path
/// during insertion. Levels cycle so ingestion doesn't hit a degenerate
/// all-same-value index, and timestamps advance monotonically so dedup
/// via raw_hash is actually per-line rather than accidentally uniform.
fn generate_lines(n: usize) -> Vec<String> {
    let levels = ["info", "warn", "error", "debug"];
    let services = ["payments", "orders", "auth", "api"];
    (0..n)
        .map(|i| {
            let level = levels[i % levels.len()];
            let service = services[i % services.len()];
            // 2026-04-15T09:00:00Z + i seconds — far in the past for any
            // "last Nh" queries, but always a valid ISO-8601 timestamp.
            let hour = 9 + (i / 3600) % 24;
            let minute = (i / 60) % 60;
            let second = i % 60;
            format!(
                r#"{{"timestamp":"2026-04-15T{hour:02}:{minute:02}:{second:02}Z","level":"{level}","message":"event {i}","service":"{service}","user_id":{i},"duration_ms":{dur}}}"#,
                dur = (i * 13) % 5000
            )
        })
