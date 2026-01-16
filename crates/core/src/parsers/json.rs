//! JSON line parser — the original v0.1 ingest path, moved into the
//! `parsers` subdirectory in v0.2.0 alongside `logfmt` and `plain`.
//!
//! This module produces a [`LogEntry`] from one line of JSON-formatted
//! input. It is invoked through the format dispatcher in
//! [`crate::parsers::parse_line`] when the caller selects
//! [`crate::parsers::LogFormat::Json`].
//!
//! Behavior is bit-for-bit identical to v0.1: malformed input is silently
//! skipped (returns `None`), known keys are lifted into struct fields, and
//! unknown keys are preserved in `LogEntry::fields` for `json_extract()`-
//! based querying downstream.

use serde_json::Value;

use crate::entry::LogEntry;

/// Parse a single line of JSON log input.
///
/// Returns `Some(LogEntry)` if `line` is a non-empty JSON object, otherwise
/// `None`. Callers iterate over an input source and discard `None` results
/// (optionally incrementing a "lines skipped" counter — the CLI does this
/// during ingestion).
///
/// # Behaviour
///
/// - Empty or whitespace-only lines return `None`.
/// - Lines that are not valid JSON return `None`.
/// - Lines that are valid JSON but not objects (e.g. `42`, `"hi"`, `[1,2]`)
///   return `None`, because logdive's ingestion contract restricts JSON
///   input to top-level objects.
/// - Within an object, keys matching [`LogEntry::KNOWN_KEYS`] populate the
///   corresponding struct fields; all other keys go into `LogEntry::fields`.
/// - For the known string-typed fields, non-string scalar values (numbers,
///   booleans, null) are stringified so information is preserved. Object
///   and array values for known fields are *not* coerced — instead they
///   remain in `fields` under their original key, leaving the known field
///   as `None`.
pub fn parse_line(line: &str) -> Option<LogEntry> {
    if line.trim().is_empty() {
        return None;
    }

    let value: Value = serde_json::from_str(line).ok()?;
    let obj = match value {
        Value::Object(map) => map,
        _ => return None,
    };

    let mut entry = LogEntry::new(line);

    for (key, value) in obj {
        match key.as_str() {
            "timestamp" => match coerce_scalar_to_string(&value) {
                Some(s) => entry.timestamp = Some(s),
                None => {
                    entry.fields.insert(key, value);
                }
            },
            "level" => match coerce_scalar_to_string(&value) {
                Some(s) => entry.level = Some(s),
                None => {
                    entry.fields.insert(key, value);
                }
            },
            "message" => match coerce_scalar_to_string(&value) {
                Some(s) => entry.message = Some(s),
                None => {
                    entry.fields.insert(key, value);
                }
            },
            "tag" => match coerce_scalar_to_string(&value) {
                Some(s) => entry.tag = Some(s),
                None => {
                    entry.fields.insert(key, value);
                }
            },
            _ => {
                entry.fields.insert(key, value);
            }
        }
    }

    Some(entry)
}
