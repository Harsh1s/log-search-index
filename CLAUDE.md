# CLAUDE.md — Working with logdive

This file orients Claude Code (and any AI assistant) before touching the codebase.
Read this first. The rules here override any defaults.

## What this project is

logdive is a fast, self-hosted query engine for structured logs. Three-crate
Rust workspace (edition 2024, MSRV 1.85). Ships two binaries:

- `logdive` — CLI: ingest, query, stats, prune.
- `logdive-api` — read-only HTTP server: GET /query, /stats, /version.

Library half: `logdive-core` is publishable as a standalone crate.

Current version: **0.3.0** (released 2026-06-05).
Next milestone: **0.4.0** (yaml/csv output, configurable retention by source, Windows --follow).

## Commands

```bash
# Compile checkpoint — run after every meaningful change
cargo build --workspace
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Run binaries
cargo run --bin logdive -- --help
cargo run --bin logdive-api -- --help

# Benchmarks
cargo bench   # reports in target/criterion/

# Pre-release verification (full battery)
./scripts/prerelease-check.sh
```

## Crate layout

```
crates/
  core/   — logdive-core: library (LogEntry, parsers, Indexer, query, executor, FileTailer)
  cli/    — logdive: binary (subcommands: ingest, query, stats, prune)
  api/    — logdive-api: binary (axum router, AppState, handlers)
```

