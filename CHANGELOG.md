# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-05

### Added

- **M1 — Parenthesized query groups**
  - `(` `)` grouping in query language: `(level=error OR level=warn) AND service=payments`.
  - `Clause::Group(Box<QueryNode>)` variant added to query AST; parser and executor updated.

- **M2 — CLI pagination and output flag rename**
  - `logdive query --limit N` (unchanged default 1000; `0` = unlimited).
  - `logdive query --offset N` (new; skips leading results for page navigation).
  - `--format` on `logdive query` renamed to `--output` (`pretty|json`) to disambiguate from `ingest --format` (input format). **Breaking CLI flag change.**
  - `QueryOptions { limit, offset }` struct in `logdive-core` replaces bare `limit: Option<usize>` on `execute` / `execute_at`. **Breaking library change.**

- **M3 — HTTP pagination**
  - `GET /query?offset=<n>` query parameter added to `logdive-api`. Mirrors CLI `--offset` semantics: absent or `0` starts from the first result.

- **M4 — Case-insensitive level queries**
  - Expression index `idx_level_norm ON log_entries(lower(level))` added to schema; idempotent — existing databases pick it up on next `Indexer::open()`.
  - `level=ERROR`, `level=Error`, and `level=error` all match the same rows.
  - `level contains "ERR"` matches rows whose stored level is `"error"`.

- **M5 — Distroless Docker runtime**
  - Runtime stage changed from `debian:bookworm-slim` to `gcr.io/distroless/cc-debian12:nonroot`.
  - No shell, no curl, no root user in the final image.
  - `logdive-api --health-check` flag: TCP-connects to own port via stdlib `TcpStream` and exits 0/1 — works in distroless without any shell or HTTP client.
  - `HEALTHCHECK CMD ["/usr/local/bin/logdive-api", "--health-check"]` replaces the previous curl-based check.

### Breaking (library)

- `execute(query, conn, opts: QueryOptions)` — third parameter changed from `Option<usize>` (limit only) to `QueryOptions { limit, offset }` (M2).
- `execute_at(query, conn, opts: QueryOptions, now)` — same change (M2).

### Breaking (CLI)

- `logdive query --format` renamed to `logdive query --output` (M2).

## [0.2.1] - 2026-06-01

### Added

- **H1 — Security tests** (`crates/core/tests/security.rs`, 10 tests)
  - SQL injection via field name: tokenizer rejects `'`, `;`, Unicode
    lookalikes (`U+2019`) — parse fails before any SQL is generated.
  - SQL injection via value: bound parameters prevent `DROP TABLE` and
    `1=1` tautology payloads from affecting results.
  - LIKE wildcard escaping: `_` and `\` in `contains` queries match
    literally, not as SQL wildcards or escape characters.
  - Resource exhaustion: 1 000-disjunct OR query parses and executes
    without stack overflow (iterative `parse_or_expr`). 10 MB raw line
    ingested without panic or OOM.

- **H2 — Functional tests** (28 new tests across 7 suites)
  - Property-based (`proptest`): arbitrary input never panics; valid
    equality queries produce single-group ASTs; OR disjunct count matches
    input; printable ASCII in quoted values never panics.
  - Cross-format dedup: same raw line ingested twice is one row; JSON
    vs. logfmt with identical logical content are two distinct rows.
  - Concurrent CLI ingest: two `logdive ingest` processes on the same
    database produce no corruption and dedup is respected.
  - Parser edge cases: UTF-8 BOM prefix rejected; deeply nested object
    in a known field preserved in `fields` map; whitespace in known
    field value preserved verbatim.
  - Time-range: space-separator datetime accepted; boundary row at
