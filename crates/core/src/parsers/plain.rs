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

    let mut entry = LogEntry::new(line);
    entry.message = Some(line.to_string());
    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_line_becomes_message() {
        let e = parse_line("starting service").expect("should parse");
        assert_eq!(e.message.as_deref(), Some("starting service"));
    }

    #[test]
    fn empty_line_returns_none() {
        assert!(parse_line("").is_none());
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert!(parse_line("   \t  ").is_none());
    }

    #[test]
    fn unicode_content_preserved() {
        let e = parse_line("正在启动 service - café").expect("should parse");
        assert_eq!(e.message.as_deref(), Some("正在启动 service - café"));
    }

    #[test]
    fn internal_whitespace_preserved() {
        let e = parse_line("hello\tworld   foo").expect("should parse");
        assert_eq!(e.message.as_deref(), Some("hello\tworld   foo"));
    }

    #[test]
    fn leading_and_trailing_whitespace_preserved() {
        // Explicit lock: we don't trim. The full original byte content is
        // preserved both in `message` and `raw` so dedup hashing matches
        // exactly what the user fed in.
