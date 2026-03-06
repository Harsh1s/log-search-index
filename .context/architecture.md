# architecture.md

## Crate map

```
crates/
  core/                        logdive-core — publishable library
    src/
      lib.rs                   Public re-exports; gated follow module
      entry.rs                 LogEntry struct + KNOWN_KEYS + with_tag()
      error.rs                 LogdiveError (thiserror), Result alias
      indexer.rs               Indexer (SQLite handle), InsertStats, PruneStats, Stats
                               db_path(), BATCH_SIZE=1000
      query.rs                 Tokenizer, AST (QueryNode/AndGroup/Clause), parse()
      executor.rs              build_sql(), translate_clause(), execute(), execute_at()
                               QueryOptions { limit: Option<usize>, offset: Option<usize> }
      parsers/
        mod.rs                 LogFormat enum, LogFormat::ALL, parse_line() dispatcher
        json.rs                JSON-object-per-line parser
        logfmt.rs              logfmt key=value parser (hand-written tokenizer)
        plain.rs               Whole-line → message parser
      follow.rs                FileTailer — Unix-only, #[cfg(unix)]
    tests/
      cross_format.rs          5 integration tests: dedup across formats
      proptest_query.rs        4 property-based tests: parser never panics
      security.rs              10 security tests: SQLi, LIKE injection, resource exhaustion
      functional.rs            28 functional tests: time-range, follow, API, prune boundary
    benches/
      bench_ingest.rs          Criterion: insert throughput
      bench_query.rs           Criterion: query latency at scale

  cli/                         logdive — CLI binary (crate name: logdive)
    src/
      main.rs                  Clap struct, handle_ingest, handle_query, run_watch_loop
      render.rs                OutputFormat (Pretty/Json), render(), ANSI + NO_COLOR
      stats_cmd.rs             run_stats() — wraps Indexer::stats(), formats output
      prune_cmd.rs             run_prune() — cutoff parsing, confirmation prompt
    tests/
      concurrent.rs            2 tests: two processes writing same DB simultaneously

  api/                         logdive-api — HTTP binary (crate name: logdive-api)
    src/
      lib.rs                   Module re-exports for test access
      main.rs                  Clap, ensure_index_exists, parse_cors_origins, axum::serve
                               --health-check flag: TcpStream::connect to own port, exit 0/1
      router.rs                build_router(), build_cors_layer()
      handlers.rs              query_handler, stats_handler, version_handler
                               StatsResponse, VersionResponse, QueryParams { q, limit, offset }
      error.rs                 AppError (maps LogdiveError → HTTP status codes)
      state.rs                 AppState { db_path }, with_connection()
    tests/
      integration.rs           21 end-to-end HTTP tests via tower::ServiceExt::oneshot
```

## Locked decisions

**SQLite via rusqlite (bundled)**
- What: single embedded database, no external process
- Why: zero runtime dependencies; ships inside the binary; battle-tested at
  internet scale; adequate performance for single-host log volume
- What breaks if changed: everything — all storage, query, dedup logic is SQL

**Hybrid schema — known fields as columns, rest in JSON blob**
- What: `timestamp`, `level`, `message`, `tag` are real indexed columns;
  everything else is serialized into `fields TEXT` and queried via
  `json_extract()`
- Why: indexed columns give sub-millisecond point lookups on the four most
  common filter fields; JSON blob makes the schema open — any log shape ingests
  without migration
- What breaks if changed: the executor's `column_for_field()` routing, all
  existing indexes, all existing stored data

**Hand-written recursive descent query parser**
- What: ~400 lines in `query.rs`; no parser-combinator library
- Why: the grammar is small and stable; a combinator library adds a compile-time
