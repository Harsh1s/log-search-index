//! The `LogEntry` type — a single parsed log line.
//!
//! This is the canonical in-memory representation used by every layer of
//! logdive: the parser produces `LogEntry` values, the indexer consumes them,
//! and the executor returns them from queries.
//!
//! Field layout follows the hybrid storage decision in the project doc
//! (decisions log, 2026-04-19): the four "known" fields (`timestamp`, `level`,
//! `message`, `tag`) are first-class members that map to indexed SQLite
//! columns, while everything else lives in the `fields` map and is persisted
//! as a JSON blob queried via `json_extract()`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A single structured log entry.
///
/// All four known fields are `Option<String>` because a JSON line may omit
/// any of them and still be worth indexing — the parser's contract per the
/// milestone 1 spec is "missing optional fields use `None` not panic".
///
/// `fields` holds only keys that are *not* among the known four. The parser
/// is responsible for lifting known keys out of the JSON object and into
/// their dedicated fields before populating this map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// The entry's timestamp as it appeared in the source line. Stored as a
    /// string rather than a parsed `DateTime` so unusual formats survive
    /// ingestion; time-range filtering is applied at query time.
    pub timestamp: Option<String>,

    /// Log severity ("error", "warn", "info", ...). Indexed column.
    pub level: Option<String>,

    /// The human-readable message body.
    pub message: Option<String>,

    /// Optional source tag supplied at ingestion time (the `--tag` CLI flag,
    /// per the "Decisions log" entry on optional tags). `None` means
    /// "untagged" and such entries match queries without a tag filter.
    pub tag: Option<String>,

    /// Arbitrary additional keys from the JSON object. Serialized to the
    /// `fields TEXT` column and queried via SQLite's `json_extract()`.
    pub fields: Map<String, Value>,
