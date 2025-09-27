//! Security-focused integration tests for the logdive query pipeline.
//!
//! Tests use only the public `logdive-core` API. The per-module unit tests
//! (`executor::tests`, `indexer::tests`, `query::tests`) probe private helpers
//! (`is_safe_json_path_segment`, `validate_field_name`, etc.) directly; these
//! tests verify the outer wall — what a caller of the library experiences when
//! passing adversarial input through the full parse → execute path.

use logdive_core::{Indexer, LogEntry, QueryOptions, execute, parse_query};

fn make_entry(ts: &str, level: &str, message: &str) -> LogEntry {
    let raw = format!(r#"{{"timestamp":"{ts}","level":"{level}","message":"{message}"}}"#);
    let mut e = LogEntry::new(raw);
    e.timestamp = Some(ts.to_string());
    e.level = Some(level.to_string());
    e.message = Some(message.to_string());
    e
}

// ── SQL injection via field name ─────────────────────────────────────────────

#[test]
fn field_with_single_quote_is_parse_error() {
    // The tokenizer hits `'` before the field name reaches validate_field_name;
    // parse must fail and never reach SQL generation.
    assert!(parse_query("serv'ice=x").is_err());
}

#[test]
fn field_with_semicolon_is_parse_error() {
    assert!(parse_query("a;b=x").is_err());
}

#[test]
fn field_with_or_injection_payload_is_parse_error() {
    // Classic injection shape: tokenizer rejects `'` at byte 7.
    assert!(parse_query("service' OR 1=1--=x").is_err());
}

#[test]
fn field_with_unicode_non_ascii_is_parse_error() {
    // U+2019 RIGHT SINGLE QUOTATION MARK looks like a quote. The tokenizer
    // only allows ASCII bytes in identifiers; the multi-byte UTF-8 sequence
    // hits the catch-all error branch.
    assert!(parse_query("svc\u{2019}=x").is_err());
}

// ── SQL injection via value ──────────────────────────────────────────────────

#[test]
fn value_injection_does_not_drop_table() {
    let mut idx = Indexer::open_in_memory().unwrap();
