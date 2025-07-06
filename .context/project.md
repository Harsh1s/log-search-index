# project.md

## Identity

- **Name**: logdive
- **Repository**: https://github.com/Aryagorjipour/logdive
- **License**: MIT OR Apache-2.0
- **crates.io**: https://crates.io/crates/logdive (CLI), https://crates.io/crates/logdive-api, https://crates.io/crates/logdive-core
- **Docs**: https://docs.rs/logdive-core
- **Docker registry**: ghcr.io/aryagorjipour/logdive
- **Landing page**: https://aryagorjipour.github.io/logdive/ (GitHub Pages, Astro 5)
- **Current version**: 0.3.0 (released 2026-06-05)
- **Next planned**: v0.4.0 (performance / benchmarks / speed focus)
- **Long-term target**: v1.0.0 (stable API + complete docs) → triggers Show HN

## What it is

logdive is a single-binary structured log query engine. You point it at a JSON,
logfmt, or plain-text log file; it parses and indexes every line into a local
SQLite database; you query with a typed expression language that supports AND,
OR, parenthesised groups, field comparisons, substring search, and relative or
absolute time ranges. A companion binary (`logdive-api`) exposes the same query
capability as a read-only HTTP server. No infrastructure, no daemons, no cloud
accounts.

## Target user

Backend engineer running a side project or small team service. They have logs
on disk or piped from Docker. They want to filter, search, and time-range-slice
them faster than `grep | jq` chains allow, without spinning up Loki, Datadog,
or Elastic — all of which require infrastructure they don't have, cost they
don't want, or configuration they don't have time for. They're comfortable with
