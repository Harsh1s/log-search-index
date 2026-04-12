//! logfmt line parser.
//!
//! Parses lines in the [logfmt](https://brandur.org/logfmt) format — a
//! sequence of `key=value` pairs separated by ASCII whitespace, pioneered
//! by Heroku and widely adopted by Go services via `go-kit/log`.
//!
//! # Grammar
//!
//! ```text
//! line   := pair (whitespace+ pair)*
//! pair   := key (= value)?
//! key    := [a-zA-Z_] [a-zA-Z0-9_.-]*
//! value  := bare_value | quoted_value
//! bare_value   := any chars up to the next whitespace
//! quoted_value := '"' (escaped_char | not_quote)* '"'
//! escaped_char := \\ \\ | \\ "
//! ```
//!
//! # Semantics
//!
//! - **Bareword booleans**: a key with no `=` is stored as `key=true`,
//!   matching go-kit and Heroku conventions. So `level=info debug` parses
//!   as two pairs: `level=info` and `debug=true`.
//!
//! - **Empty values**: `key=` (with trailing whitespace or end-of-line)
//!   stores an empty string. Distinct from `key` alone, which is a
//!   bareword boolean.
//!
//! - **Duplicate keys**: last write wins. This matches go-kit's logfmt
//!   parser and Heroku's behavior.
//!
//! - **No type coercion**: every value is stored as a string. Unlike the
//!   JSON parser, which preserves native types from the source, logfmt
//!   has no native typing — `duration_ms=1234` arrives as the string
//!   `"1234"`. Numeric comparisons like `duration_ms > 1000` will compare
//!   lexically rather than numerically against logfmt-ingested data.
//!   Users needing typed comparisons should ingest as JSON.
//!
//! - **Lenient on malformed tokens**: a token that doesn't start with a
//!   valid key character (letter or underscore) is skipped to the next
//!   whitespace boundary; surrounding pairs are kept. The whole line is
//!   only dropped (returns `None`) on truly fatal conditions: empty
//!   input, no parseable pairs, or an unterminated quoted value.
//!
//! - **Quote escapes**: only `\"` and `\\` are recognized. Any other
//!   `\X` sequence inside a quoted value is preserved as the literal
//!   two characters `\` and `X`. This is more forgiving than a strict
//!   reading and matches what real-world logfmt emitters tend to produce.
//!
//! Known keys (`timestamp`, `level`, `message`, `tag`) are lifted into
//! the corresponding `LogEntry` struct fields after the pair list is
//! built. All other keys land in `LogEntry::fields` as
//! `serde_json::Value::String(_)`.

use serde_json::Value;

use crate::entry::LogEntry;

/// Parse a single line of logfmt input.
///
/// Returns `Some(LogEntry)` on success, `None` for empty lines, lines
/// containing no parseable pairs, or lines with an unterminated quoted
/// value. Caller iterates over input and discards `None` results,
/// counting them in the malformed-line tally.
pub fn parse_line(line: &str) -> Option<LogEntry> {
    if line.trim().is_empty() {
        return None;
    }

    let pairs = parse_pairs(line)?;
    if pairs.is_empty() {
        return None;
    }

    let mut entry = LogEntry::new(line);

    for (key, value) in pairs {
        // Known-key promotion. Last write wins for both known and unknown
        // keys — these direct assignments and the Map::insert below both
        // overwrite prior values.
        match key.as_str() {
            "timestamp" => entry.timestamp = Some(value),
            "level" => entry.level = Some(value),
            "message" => entry.message = Some(value),
            "tag" => entry.tag = Some(value),
            _ => {
                entry.fields.insert(key, Value::String(value));
            }
        }
    }

    Some(entry)
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/// Scan a line into `(key, value)` pairs.
///
/// Returns `None` only on fatal conditions (currently: unterminated
/// quoted value). On malformed tokens within an otherwise-valid line,
/// the malformed token is skipped to the next whitespace and the scan
/// continues — so the surrounding well-formed pairs are preserved.
fn parse_pairs(line: &str) -> Option<Vec<(String, String)>> {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut pairs: Vec<(String, String)> = Vec::new();

    while i < bytes.len() {
        // Skip leading whitespace before the next token.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // First byte of the token must be a valid key-start char. If it
        // isn't, skip the token to the next whitespace and continue —
        // lenient behavior to keep good pairs around bad tokens.
        if !is_key_start(bytes[i]) {
            i = skip_to_whitespace(bytes, i);
            continue;
        }

        // Read the key.
        let key_start = i;
        while i < bytes.len() && is_key_continue(bytes[i]) {
            i += 1;
        }
        // Slicing UTF-8 along ASCII boundaries is safe; the key is
        // guaranteed ASCII by `is_key_continue`.
        let key = std::str::from_utf8(&bytes[key_start..i])
            .expect("key is ASCII")
            .to_string();

        // What comes next decides the pair shape:
        //   - end of input or whitespace → bareword boolean (key=true)
        //   - `=` → read a value (bare or quoted)
        //   - anything else → malformed; skip token, drop key
        if i >= bytes.len() || bytes[i].is_ascii_whitespace() {
            pairs.push((key, "true".to_string()));
            continue;
        }
        if bytes[i] != b'=' {
            // Malformed key suffix (e.g. "key:value" or "key>v"). Skip
            // the rest of this token; the key we just collected is
            // discarded because a key without `=` and without surrounding
            // whitespace isn't a valid bareword boolean either.
            i = skip_to_whitespace(bytes, i);
            continue;
        }
        i += 1; // consume '='

        // After `=`, an immediate whitespace or EOL produces an empty
        // value (distinct from a bareword boolean).
        if i >= bytes.len() || bytes[i].is_ascii_whitespace() {
            pairs.push((key, String::new()));
            continue;
        }

        // Quoted or bare value?
        if bytes[i] == b'"' {
            i += 1; // consume opening quote
            // Unterminated quote is fatal for the whole line — bail out
            // via `?` and let the caller drop the entry.
            let value = read_quoted_value(bytes, &mut i)?;
            pairs.push((key, value));
        } else {
            let value_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            // The slice may contain non-ASCII bytes (Unicode in values);
            // it's still valid UTF-8 because the original input was &str.
            let value = std::str::from_utf8(&bytes[value_start..i])
                .expect("UTF-8 input slice along ASCII boundaries is UTF-8")
                .to_string();
            pairs.push((key, value));
        }
    }

    Some(pairs)
}

