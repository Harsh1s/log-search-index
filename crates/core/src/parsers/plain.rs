//! Plain-text line parser.
//!
//! For unstructured logs where every line is just human-readable content.
//! The entire line becomes `LogEntry::message`; `timestamp`, `level`, and
//! `tag` are all `None`; the `fields` map is empty.
//!
//! # Why no timestamp parsing
//!
//! Plaintext log files use a long tail of timestamp formats — RFC3339,
//! syslog (`Jan 1 12:34:56`), Apache combined log format, custom
//! application-specific formats. Any heuristic would be wrong some of the
//! time, and a wrong timestamp silently corrupts `last Nh` and `since`
//! queries against ingested data. The honest answer is to surface "this
//! row has no timestamp" and let the existing skip-no-timestamp policy in
//! the indexer take effect.
//!
//! Users with plaintext logs typically want the `--timestamp-now` flag on
//! `logdive ingest` (added in v0.2.0), which stamps the current ingestion
//! time onto any row without a parsed timestamp. The flag is universal
//! across formats but is most useful here.
//!
//! # Why no level extraction
//!
//! For the same reason: every plaintext format encodes severity
//! differently (`[ERROR]`, `ERROR:`, `<3>`, `severity=error`, etc.).
//! Extracting it heuristically from plaintext would couple this parser
//! to one convention. Users who want structured level filtering should
//! ingest as JSON or logfmt instead.

use crate::entry::LogEntry;

/// Parse a single line of plain-text input.
///
/// Returns `Some(LogEntry)` whose `message` is the entire line and `raw`
/// is the line preserved verbatim. Returns `None` on empty or
/// whitespace-only lines, matching the graceful-skip philosophy of the
/// other parsers.
pub fn parse_line(line: &str) -> Option<LogEntry> {
    if line.trim().is_empty() {
        return None;
    }

