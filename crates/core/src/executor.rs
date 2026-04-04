//! Query executor: translate a [`QueryNode`] into parameterized SQL, run
//! it against the index, and reconstruct [`LogEntry`] values from the
//! result rows.
//!
//! This module is the bridge between the query AST and the SQLite schema.
//! It never mixes user-controlled strings into SQL text — every literal
//! value is bound as a parameter. The one exception is JSON extraction
//! paths like `$.service`, which embed the field name directly because
//! SQLite parameters aren't allowed inside `json_extract` path expressions;
//! safety there comes from the field name having passed
//! `validate_field_name`'s strict regex in the parser, which we
//! defensively re-check at the executor boundary.
//!
//! # Disjunction (OR) shape
//!
//! v0.2.0 introduced OR. The AST is `QueryNode::Or(Vec<AndGroup>)` where
//! each [`AndGroup`] is a conjunction of clauses. The SQL we emit
//! parenthesizes each AND-group and joins them with ` OR `:
//!
//! ```sql
//! WHERE (level = ? AND json_extract(fields, '$.service') = ?)
//!    OR (level = ?)
//! ```
//!
//! For queries with no OR (a single AND-group), the parens are still
//! emitted — the alternative is a special-case branch in the SQL
//! builder that adds maintenance cost without performance benefit.
//! SQLite's planner ignores redundant parens.
//!
//! # Parenthesized groups (v0.3.0)
//!
//! `Clause::Group` wraps an inner `QueryNode::Or` subtree produced by a
//! `(` … `)` expression in the query language. The translator recurses
//! into the subtree via `translate_and_group` and parenthesizes the
//! result so it composes correctly with surrounding AND/OR operators:
//!
//! ```sql
//! -- (level=error OR level=warn) AND service=payments
//! WHERE (((level = ?) OR (level = ?)) AND json_extract(fields, '$.service') = ?)
//! ```
//!
//! The extra level of parentheses is redundant for correctness but keeps
//! the emitter uniform — every AND-group is parenthesized, whether it
//! came from the top-level OR or from an inner Group clause.
//!
//! # Pagination (v0.3.0)
//!
//! [`QueryOptions`] bundles `limit` and `offset` so callers can request a
//! specific page of results without separate function variants. Offset
//! without limit uses `LIMIT -1` — SQLite requires a `LIMIT` clause when
//! `OFFSET` is present; `-1` means unlimited in SQLite.
//!
//! # Timestamp handling
//!
//! Timestamps are compared as TEXT, which works correctly for any ISO-8601
//! format because those sort lexicographically in chronological order when
//! all components are fixed-width. Ingested timestamps that aren't ISO-8601
//! shaped will compare incorrectly against `last`/`since` bounds — a known
//! limitation of accepting arbitrary timestamp strings at ingestion time.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use rusqlite::{Connection, params_from_iter, types::Value as SqlValue};
use serde_json::{Map, Value};

use crate::entry::LogEntry;
use crate::error::{LogdiveError, Result};
use crate::query::{AndGroup, Clause, Duration, QueryNode, QueryValue};

// ---------------------------------------------------------------------------
// QueryOptions
// ---------------------------------------------------------------------------

/// Options controlling result set size and starting position for [`execute`].
///
/// `limit = None` means unlimited rows. `offset = None` means start from
/// the first result. When offset is set without a limit the SQL uses
/// `LIMIT -1` — SQLite requires a `LIMIT` clause whenever `OFFSET` appears;
/// `-1` is the SQLite convention for "no cap".
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryOptions {
    /// Maximum number of rows to return. `None` = unlimited.
    pub limit: Option<usize>,
    /// Number of rows to skip from the front of the ordered result set.
    /// `None` (or `Some(0)`) starts from the first row.
    pub offset: Option<usize>,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Execute a parsed query against the index and return matching entries.
///
/// Results are ordered by `timestamp DESC, id DESC` (newest first, with
/// row id as stable tiebreaker for identical timestamps). Use
/// [`QueryOptions`] to cap or page the result set.
pub fn execute(query: &QueryNode, conn: &Connection, opts: QueryOptions) -> Result<Vec<LogEntry>> {
    let (sql, binds) = build_sql(query, opts, Utc::now())?;
    run(conn, &sql, &binds)
}

/// Variant of [`execute`] that uses a caller-supplied "now" value.
///
/// Exposed for testing so time-range clauses produce deterministic bounds.
pub fn execute_at(
    query: &QueryNode,
    conn: &Connection,
    opts: QueryOptions,
    now: DateTime<Utc>,
) -> Result<Vec<LogEntry>> {
    let (sql, binds) = build_sql(query, opts, now)?;
    run(conn, &sql, &binds)
}

// ---------------------------------------------------------------------------
// SQL generation
// ---------------------------------------------------------------------------

/// Intermediate representation of a bindable value, kept as an owned
/// `SqlValue` so `params_from_iter` can consume them without lifetime
/// gymnastics.
type Bind = SqlValue;

fn build_sql(
    query: &QueryNode,
    opts: QueryOptions,
    now: DateTime<Utc>,
) -> Result<(String, Vec<Bind>)> {
    let QueryNode::Or(groups) = query;

    // The parser guarantees at least one AND-group, and each AND-group has
    // at least one clause. Treat both invariants defensively here so a bug
    // upstream produces a recognizable runtime shape rather than a SQL
    // syntax error from an empty `WHERE` clause.
    let mut group_sqls: Vec<String> = Vec::with_capacity(groups.len());
    let mut binds: Vec<Bind> = Vec::new();

    for group in groups {
        let (group_sql, mut group_binds) = translate_and_group(group, now)?;
        group_sqls.push(group_sql);
        binds.append(&mut group_binds);
    }

    let where_sql = if group_sqls.is_empty() {
        // Defensive: should be unreachable given the parser contract.
        "1=1".to_string()
    } else {
        // ` OR ` between groups, each already parenthesized.
        group_sqls.join(" OR ")
    };

    let mut sql = format!(
        "SELECT timestamp, level, message, tag, fields, raw \
         FROM log_entries \
         WHERE {where_sql} \
         ORDER BY timestamp DESC, id DESC"
    );

    // SQLite requires LIMIT to be present when OFFSET is used.
    // Emit `LIMIT -1` (unlimited) when the caller wants offset-only paging.
    match (opts.limit, opts.offset) {
        (Some(lim), Some(off)) if off > 0 => {
            sql.push_str(&format!(" LIMIT {lim} OFFSET {off}"));
        }
        (Some(lim), _) => {
            sql.push_str(&format!(" LIMIT {lim}"));
        }
        (None, Some(off)) if off > 0 => {
            sql.push_str(&format!(" LIMIT -1 OFFSET {off}"));
        }
        _ => {}
    }

    Ok((sql, binds))
}

/// Translate one AND-group into a parenthesized SQL fragment and the
/// associated bind values, in clause-declaration order.
///
/// Always parenthesizes — including the single-clause case. Uniformity in
/// the SQL emitter outweighs prettiness in the rare query-debugger view.
fn translate_and_group(group: &AndGroup, now: DateTime<Utc>) -> Result<(String, Vec<Bind>)> {
    let mut clause_sqls: Vec<String> = Vec::with_capacity(group.clauses.len());
    let mut binds: Vec<Bind> = Vec::new();

    for clause in &group.clauses {
        let (sql, mut clause_binds) = translate_clause(clause, now)?;
        clause_sqls.push(sql);
        binds.append(&mut clause_binds);
    }

    let inner = if clause_sqls.is_empty() {
        // Defensive: parser guarantees non-empty AND-groups.
        "1=1".to_string()
    } else {
        clause_sqls.join(" AND ")
    };
    Ok((format!("({inner})"), binds))
}

fn translate_clause(clause: &Clause, now: DateTime<Utc>) -> Result<(String, Vec<Bind>)> {
    match clause {
        Clause::Compare { field, op, value } => {
            // Route `level` through lower() so queries hit the idx_level_norm
            // expression index and match case-insensitively (ERROR == error).
            let (column_expr, bind) = if field == "level" {
                let lowered = match value {
                    QueryValue::String(s) => SqlValue::Text(s.to_lowercase()),
                    other => value_to_bind(other),
                };
                ("lower(level)".to_string(), lowered)
            } else {
                (column_for_field(field)?, value_to_bind(value))
            };
            let sql = format!("{column_expr} {op} ?");
            Ok((sql, vec![bind]))
        }
        Clause::Contains { field, value } => {
            // Escape SQL LIKE metacharacters (%, _, \) so a user searching
            // for a literal '%' doesn't accidentally wildcard the world.
            // For `level`, lowercase the pattern so it matches the lower()
            // column expression used in the index.
            let (column_expr, normalised_value) = if field == "level" {
                ("lower(level)".to_string(), value.to_lowercase())
            } else {
                (column_for_field(field)?, value.clone())
            };
            let escaped = escape_like(&normalised_value);
            let pattern = format!("%{escaped}%");
            let sql = format!("{column_expr} LIKE ? ESCAPE '\\'");
            Ok((sql, vec![SqlValue::Text(pattern)]))
        }
        Clause::LastDuration(d) => {
            let cutoff = compute_last_cutoff(*d, now);
            Ok((
                "timestamp >= ?".to_string(),
                vec![SqlValue::Text(cutoff.to_rfc3339())],
            ))
        }
        Clause::SinceDatetime(s) => {
            let dt = parse_datetime(s)?;
            Ok((
                "timestamp >= ?".to_string(),
                vec![SqlValue::Text(dt.to_rfc3339())],
            ))
        }
        Clause::Group(inner) => {
            // Recurse into the parenthesized subexpression. Each inner
            // AND-group is already parenthesized by `translate_and_group`;
            // multiple groups are joined with ` OR ` inside an extra pair
            // of parens so the whole group composes correctly with the
            // surrounding AND expression.
            let QueryNode::Or(groups) = inner.as_ref();
            let mut group_sqls: Vec<String> = Vec::with_capacity(groups.len());
            let mut binds: Vec<Bind> = Vec::new();
            for group in groups {
                let (gsql, mut gbinds) = translate_and_group(group, now)?;
                group_sqls.push(gsql);
                binds.append(&mut gbinds);
            }
            let sql = if group_sqls.len() == 1 {
                group_sqls.into_iter().next().unwrap()
            } else {
                format!("({})", group_sqls.join(" OR "))
            };
            Ok((sql, binds))
        }
    }
}

/// Return the SQL expression that references a given query field.
///
/// Known fields resolve to indexed columns. Unknown fields resolve to a
/// `json_extract(fields, '$.<field>')` expression — which is why the
/// field name must survive `validate_field_name`'s regex *and* the
/// defensive check here.
fn column_for_field(field: &str) -> Result<String> {
    if LogEntry::KNOWN_KEYS.contains(&field) {
        Ok(field.to_string())
    } else {
        if !is_safe_json_path_segment(field) {
            return Err(LogdiveError::UnsafeFieldName(field.to_string()));
        }
        Ok(format!("json_extract(fields, '$.{field}')"))
    }
}

/// Defensive: the parser's `validate_field_name` already enforces this,
/// but we re-check at the SQL boundary so the trust model is obvious
/// from inside this module. Allowed: letters, digits, `_`, `.`.
fn is_safe_json_path_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn value_to_bind(v: &QueryValue) -> Bind {
    match v {
        QueryValue::String(s) => SqlValue::Text(s.clone()),
        QueryValue::Integer(n) => SqlValue::Integer(*n),
        QueryValue::Float(f) => SqlValue::Real(*f),
        QueryValue::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
    }
}

/// Pre-escape SQL LIKE wildcards (`%`, `_`) and the escape character
/// itself so a user's literal CONTAINS string is matched literally.
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn compute_last_cutoff(d: Duration, now: DateTime<Utc>) -> DateTime<Utc> {
    // `amount` is u64; promote to i64 for chrono. Saturate on the
    // (astronomically unlikely) overflow case.
    let amount_i64 = i64::try_from(d.amount).unwrap_or(i64::MAX);
    let secs = amount_i64.saturating_mul(d.unit.seconds());
    let delta = chrono::Duration::seconds(secs);
    now.checked_sub_signed(delta).unwrap_or_else(|| {
        Utc.timestamp_opt(0, 0)
            .single()
            .expect("unix epoch is valid")
    })
}

/// Accept three datetime formats for `since` clauses:
///   - RFC3339 / ISO-8601 with timezone (e.g. `2024-01-01T10:00:00Z`)
///   - ISO naive datetime (e.g. `2024-01-01 10:00:00` or `2024-01-01T10:00:00`), interpreted as UTC
///   - ISO date (e.g. `2024-01-01`), interpreted as UTC midnight
fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    for fmt in &["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(Utc.from_utc_datetime(&ndt));
        }
    }
    if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0).expect("00:00:00 is valid");
        return Ok(Utc.from_utc_datetime(&ndt));
    }
    Err(LogdiveError::InvalidDatetime {
        input: s.to_string(),
        reason: "expected RFC3339, `YYYY-MM-DD HH:MM:SS`, or `YYYY-MM-DD`".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

fn run(conn: &Connection, sql: &str, binds: &[Bind]) -> Result<Vec<LogEntry>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params_from_iter(binds.iter()), |row| {
        let timestamp: Option<String> = row.get(0)?;
        let level: Option<String> = row.get(1)?;
        let message: Option<String> = row.get(2)?;
        let tag: Option<String> = row.get(3)?;
        let fields_json: String = row.get(4)?;
        let raw: String = row.get(5)?;
        // We tunnel the raw JSON out; deserialization happens below so the
        // closure's error type stays `rusqlite::Error`.
        Ok((timestamp, level, message, tag, fields_json, raw))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (timestamp, level, message, tag, fields_json, raw) = row?;
        let fields: Map<String, Value> =
            serde_json::from_str(&fields_json).map_err(LogdiveError::CorruptFieldsJson)?;
        out.push(LogEntry {
            timestamp,
            level,
            message,
            tag,
            fields,
            raw,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::Indexer;
    use crate::query::parse;
    use std::collections::HashSet;

    /// Convenience: parse a query string and run it against the given
    /// connection. Panics if parsing fails — tests pass well-formed input.
    fn run_query(conn: &Connection, q: &str) -> Vec<LogEntry> {
        let ast = parse(q).expect("test queries are well-formed");
        execute(&ast, conn, QueryOptions::default()).expect("execute")
    }

    fn run_query_opts(conn: &Connection, q: &str, opts: QueryOptions) -> Vec<LogEntry> {
        let ast = parse(q).expect("test queries are well-formed");
        execute(&ast, conn, opts).expect("execute")
    }

    fn run_query_at(conn: &Connection, q: &str, now: DateTime<Utc>) -> Vec<LogEntry> {
        let ast = parse(q).expect("test queries are well-formed");
        execute_at(&ast, conn, QueryOptions::default(), now).expect("execute")
    }

    fn make_entry(ts: &str, level: &str, message: &str) -> LogEntry {
        let raw = format!(r#"{{"timestamp":"{ts}","level":"{level}","message":"{message}"}}"#);
        let mut e = LogEntry::new(raw);
        e.timestamp = Some(ts.to_string());
        e.level = Some(level.to_string());
        e.message = Some(message.to_string());
        e
    }

    fn fixture() -> Indexer {
        let mut idx = Indexer::open_in_memory().unwrap();
        let mut a = make_entry("2026-04-20T10:00:00Z", "error", "payment failed");
        a.tag = Some("api".into());
        a.fields
            .insert("service".into(), Value::String("payments".into()));
        a.fields.insert("req_id".into(), Value::from(100));

        let mut b = make_entry("2026-04-20T11:00:00Z", "info", "health check");
        b.tag = Some("api".into());
        b.fields
            .insert("service".into(), Value::String("payments".into()));
        b.fields.insert("req_id".into(), Value::from(200));

