//! Query language: tokenizer, AST, and recursive descent parser.
//!
//! Implements the grammar from the project doc's "Notes → Query language
//! grammar" section, extended in v0.2.0 to support OR and in v0.3.0 to
//! support explicit parenthesised grouping.
//!
//! This module owns *only* the parse step: `&str → QueryNode`. Translating
//! a `QueryNode` into SQL and binding parameters is the executor's job.
//! Resolving relative time ranges like `last 2h` against wall-clock time
//! is also the executor's job; the AST just carries the raw spec.
//!
//! # Grammar (v0.3.0)
//!
//! ```text
//! query     := or_expr
//! or_expr   := and_expr (OR and_expr)*
//! and_expr  := clause (AND clause)*
//! clause    := field OP value
//!            | field CONTAINS string
//!            | TIME_RANGE
//!            | "(" or_expr ")"
//! field     := [a-zA-Z_][a-zA-Z0-9_.]*
//! OP        := "=" | "!=" | ">" | "<"
//! value     := string | number | bool
//! string    := '"' .* '"' | bare_word
//! TIME_RANGE := "last" duration | "since" datetime
//! duration  := number ("m" | "h" | "d")
//! ```
//!
//! AND binds tighter than OR. With parentheses users can override precedence:
//! `(level=error OR level=warn) AND service=payments`.
//!
//! Without parens: `level=error AND service=payments OR level=warn` parses as
//! `(level=error AND service=payments) OR level=warn`.

use std::fmt;

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

/// The top-level query: a disjunction of one or more AND-groups.
///
/// A query with no OR (e.g. `level=error AND tag=api`) parses as a single-
/// element vector containing one `AndGroup`, so the executor has exactly
/// one code path. This mirrors the v0.1 design choice of always wrapping
/// a single clause in `And(vec![clause])` — the same idea, lifted up one
/// level of grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryNode {
    Or(Vec<AndGroup>),
}

/// A conjunction of one or more clauses, joined by AND.
///
/// `clauses` is guaranteed by the parser to be non-empty.
#[derive(Debug, Clone, PartialEq)]
pub struct AndGroup {
    pub clauses: Vec<Clause>,
}

/// A single clause — the atomic unit a query is built from.
#[derive(Debug, Clone, PartialEq)]
pub enum Clause {
    /// `field OP value` — e.g. `level = error`, `req_id > 100`.
    Compare {
        field: String,
        op: CompareOp,
        value: QueryValue,
    },
    /// `field CONTAINS string` — substring match on a string column.
    Contains { field: String, value: String },
    /// `last <N><unit>` — relative time range ending at query time.
    LastDuration(Duration),
    /// `since <datetime>` — absolute time range starting at the given moment.
    /// The string is opaque at the parse layer; the executor uses chrono to
    /// resolve it (which allows us to accept multiple formats without
    /// teaching the grammar about any particular one).
    SinceDatetime(String),
    /// `( or_expr )` — explicit grouping to override AND/OR precedence.
    /// New in v0.3.0.
    Group(Box<QueryNode>),
}

/// Comparison operator for `field OP value` clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    NotEq,
    Gt,
    Lt,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CompareOp::Eq => "=",
            CompareOp::NotEq => "!=",
            CompareOp::Gt => ">",
            CompareOp::Lt => "<",
        })
    }
}

/// A literal value appearing on the right-hand side of a comparison.
///
/// The type distinction matters because the executor binds numbers and
/// booleans with their native SQLite types so numeric comparison
/// (`req_id > 100`) uses proper ordering rather than lexical.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

/// A relative duration parsed from `last <N><unit>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    pub amount: u64,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Minutes,
    Hours,
    Days,
}

impl DurationUnit {
    /// Total seconds for one unit. The executor multiplies by `amount` to
    /// compute the cutoff timestamp against `now`.
    pub fn seconds(self) -> i64 {
        match self {
            DurationUnit::Minutes => 60,
            DurationUnit::Hours => 60 * 60,
            DurationUnit::Days => 24 * 60 * 60,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Parse error with a byte offset into the original input.
///
/// Byte offsets (rather than line/column) are sufficient because queries
/// are single-line. The CLI's pretty printer can slice the original input
/// around `position` to render a caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryParseError {
    pub position: usize,
    pub message: String,
}

impl fmt::Display for QueryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "query parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for QueryParseError {}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A bare identifier — could be a field name, a bare-word value, or a
    /// keyword depending on position. We resolve keywords at parse time
    /// rather than at tokenization time because "last" used as a field name
    /// (in the unlikely event a log has a field literally called "last")
    /// should still work in `CONTAINS` contexts.
    Ident(String),
    /// A double-quoted string, with the quotes stripped.
    QuotedString(String),
    /// A literal number — stored as text so the parser can decide whether
    /// it's an integer or float.
    Number(String),
    Eq,
    NotEq,
    Gt,
    Lt,
    LParen,
    RParen,
}

#[derive(Debug, Clone)]
struct SpannedToken {
    token: Token,
    position: usize,
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/// Return true if `b` is allowed *inside* an identifier (but not necessarily
/// as the first byte). Matches the grammar's field rule plus the extra
/// characters needed for bare-word values and datetime literals: `-` for
/// hyphenated values like `x-request-id`, `:` for colon-separated values
/// like time components, and `.` for both dotted field names and float-like
/// version strings in values.
fn is_ident_continuation(b: u8) -> bool {
    b == b'_' || b == b'.' || b == b'-' || b == b':' || b.is_ascii_alphanumeric()
}

/// Split the input into a stream of tokens with byte-offset positions.
///
/// Whitespace is skipped. Unrecognized bytes produce a `QueryParseError`
/// pointing at the offending character.
fn tokenize(input: &str) -> Result<Vec<SpannedToken>, QueryParseError> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();

    while i < bytes.len() {
        let c = bytes[i];

        // Whitespace.
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Operators — order matters: check `!=` before `!` would-be, and
        // both before single `<`/`>`/`=`.
        if c == b'!' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                out.push(SpannedToken {
                    token: Token::NotEq,
                    position: i,
                });
                i += 2;
                continue;
            }
            return Err(QueryParseError {
                position: i,
                message: "unexpected '!' — did you mean '!='?".to_string(),
            });
        }
        if c == b'=' {
            out.push(SpannedToken {
                token: Token::Eq,
                position: i,
            });
            i += 1;
            continue;
        }
        if c == b'>' {
            out.push(SpannedToken {
                token: Token::Gt,
                position: i,
            });
            i += 1;
            continue;
        }
        if c == b'<' {
            out.push(SpannedToken {
                token: Token::Lt,
                position: i,
            });
            i += 1;
            continue;
        }

        // Parentheses (v0.3.0).
        if c == b'(' {
            out.push(SpannedToken {
                token: Token::LParen,
                position: i,
            });
            i += 1;
            continue;
        }
        if c == b')' {
            out.push(SpannedToken {
                token: Token::RParen,
                position: i,
            });
            i += 1;
            continue;
        }

        // Quoted string.
        if c == b'"' {
            let start = i;
            i += 1; // consume opening quote
            let content_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                // No escape handling in v1 — the grammar is `'"' .* '"'`
                // and real log-query users don't embed quotes in values.
                // If this becomes a pain we add escape handling later.
                i += 1;
            }
            if i >= bytes.len() {
                return Err(QueryParseError {
                    position: start,
                    message: "unterminated quoted string".to_string(),
                });
            }
            let s = std::str::from_utf8(&bytes[content_start..i])
                .expect("input is &str, slice is UTF-8")
                .to_string();
            i += 1; // consume closing quote
            out.push(SpannedToken {
                token: Token::QuotedString(s),
                position: start,
            });
            continue;
        }

        // Digit-led token.
        //
        // Two possibilities:
        //  - Pure-digit run (with optional fractional part) → Token::Number.
        //    Example: `100`, `1.5`.
        //  - Digit-led run that contains `-` or `:` → Token::Ident. This
        //    supports bare datetime literals like `2024-01-01T10:00:00Z`
        //    after `since`. Colon is included for completeness so time-
        //    of-day literals don't need quoting either.
        //
        // The disambiguation happens at the first non-digit, non-dot byte:
        // if that byte is `-` or `:`, we promote the whole run (and keep
        // consuming continuation bytes) to an Ident. Otherwise we stop at
        // the end of the numeric run and emit a Number.
        if c.is_ascii_digit() {
            let start = i;
            let mut saw_dot = false;

            // First phase: consume digits and at most one dot (only when
            // the dot is followed by a digit, preserving the existing
            // `1.5` behaviour). We peek at the next byte after each dot
            // to decide.
            while i < bytes.len() && (bytes[i].is_ascii_digit() || (bytes[i] == b'.' && !saw_dot)) {
                if bytes[i] == b'.' {
                    if i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_digit() {
                        break;
                    }
                    saw_dot = true;
                }
                i += 1;
            }

            // Second phase: if the next byte indicates this digit-led run
            // is actually an ident (datetime, multi-dot version string),
            // keep consuming all ident-continuation bytes and emit Ident.
            //
            // Promotion triggers:
            //   `-` or `:` — datetime literals (`2024-01-01`, `10:30`)
            //   `.`        — dotted strings beyond one fractional part
            //                (`1.2.3`); the first phase stops at the
            //                second dot due to its `!saw_dot` guard,
            //                leaving `bytes[i]` on that second dot.
            //
            // Letters are intentionally NOT a promotion trigger: `30m`
            // must tokenize as Number("30") + Ident("m") so the parser's
            // `last <N><unit>` rule works. Users who want digit-led
            // values with letter suffixes (`3beta`) must quote them.
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b':' || bytes[i] == b'.') {
                while i < bytes.len() && is_ident_continuation(bytes[i]) {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[start..i])
                    .expect("input is &str, slice is UTF-8")
                    .to_string();
                out.push(SpannedToken {
                    token: Token::Ident(s),
                    position: start,
                });
                continue;
            }

            let s = std::str::from_utf8(&bytes[start..i])
                .expect("ascii digits are UTF-8")
                .to_string();
            out.push(SpannedToken {
                token: Token::Number(s),
                position: start,
            });
            continue;
        }

        // Identifier / bare word: starts with letter or underscore,
        // continues per `is_ident_continuation`. Hyphen and colon are
        // allowed inside so bare-word values like `x-request-id` and
        // colon-separated fragments work; `validate_field_name` later
        // enforces the stricter field-name subset.
        if c == b'_' || c.is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && is_ident_continuation(bytes[i]) {
                i += 1;
            }
            let s = std::str::from_utf8(&bytes[start..i])
                .expect("input is &str, slice is UTF-8")
                .to_string();
            out.push(SpannedToken {
                token: Token::Ident(s),
                position: start,
            });
            continue;
        }

        return Err(QueryParseError {
            position: i,
            message: format!("unexpected character {:?}", c as char),
        });
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse a query string into a `QueryNode`.
///
/// This is the only public entry point. Implements the v0.3.0 grammar
/// top-down via recursive descent: `or_expr` at the outermost level,
/// `and_expr` one level in, `clause` at the leaves (with optional paren
/// recursion back to `or_expr`).
pub fn parse(input: &str) -> Result<QueryNode, QueryParseError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(QueryParseError {
            position: 0,
            message: "empty query".to_string(),
        });
    }

    let mut p = Parser {
        tokens: &tokens,
        cursor: 0,
    };
    let node = p.parse_or_expr()?;

    // After a complete or_expr, the only acceptable state is end-of-input.
    // If a token remains, the user wrote something the grammar doesn't
    // accept (commonly a missing AND/OR between clauses, or an unmatched `)`).
    if let Some(extra) = p.peek() {
        let message = if matches!(extra.token, Token::RParen) {
            "unexpected ')' — no matching '('".to_string()
        } else {
            "expected 'AND' or 'OR' between clauses".to_string()
        };
        return Err(QueryParseError {
            position: extra.position,
            message,
        });
    }

    Ok(node)
}

struct Parser<'a> {
    tokens: &'a [SpannedToken],
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a SpannedToken> {
        self.tokens.get(self.cursor)
    }

    fn advance(&mut self) -> Option<&'a SpannedToken> {
        let t = self.tokens.get(self.cursor);
        if t.is_some() {
            self.cursor += 1;
        }
        t
    }

    /// Position to attribute to an error when the tokens are exhausted.
    fn end_position(&self) -> usize {
        self.tokens
            .last()
            .map(|t| t.position + token_len(&t.token))
            .unwrap_or(0)
    }

    /// Top-level: one or more AND-groups separated by `OR`.
    ///
    /// AND binds tighter than OR. We always wrap the result in
    /// `QueryNode::Or`, even for a query with no OR present — uniformity
    /// keeps the executor simple.
    fn parse_or_expr(&mut self) -> Result<QueryNode, QueryParseError> {
        let mut groups = Vec::new();
        groups.push(self.parse_and_expr()?);

        while let Some(tok) = self.peek() {
            match &tok.token {
                Token::Ident(s) if s.eq_ignore_ascii_case("or") => {
                    let or_pos = tok.position;
                    self.advance();
                    // Detect trailing-OR or doubled-OR before we recurse.
                    // Without this the inner parse_and_expr would error
                    // with a less helpful message.
                    match self.peek() {
                        None => {
                            return Err(QueryParseError {
                                position: or_pos,
                                message: "expected a clause after 'OR'".to_string(),
                            });
                        }
                        Some(next) => {
                            if let Token::Ident(s2) = &next.token {
                                if s2.eq_ignore_ascii_case("or") || s2.eq_ignore_ascii_case("and") {
                                    return Err(QueryParseError {
                                        position: next.position,
                                        message: format!(
                                            "expected a clause after 'OR', got '{}'",
                                            s2.to_uppercase()
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    groups.push(self.parse_and_expr()?);
                }
                _ => break,
            }
        }

        Ok(QueryNode::Or(groups))
    }

    /// One AND-group: one or more clauses joined by `AND`.
    fn parse_and_expr(&mut self) -> Result<AndGroup, QueryParseError> {
        let mut clauses = Vec::new();
        clauses.push(self.parse_clause()?);

        while let Some(tok) = self.peek() {
            match &tok.token {
                Token::Ident(s) if s.eq_ignore_ascii_case("and") => {
                    let and_pos = tok.position;
                    self.advance();
                    // Same trailing/double check as for OR.
                    match self.peek() {
                        None => {
                            return Err(QueryParseError {
