<p align="center">
  <img src="assets/banner.png" alt="logdive — query your logs. no daemon. no yaml." width="700" />
</p>

<p align="center">
  <a href="https://github.com/Aryagorjipour/logdive/actions/workflows/ci.yml"><img src="https://github.com/Aryagorjipour/logdive/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/Aryagorjipour/logdive/actions/workflows/docker.yml"><img src="https://github.com/Aryagorjipour/logdive/actions/workflows/docker.yml/badge.svg" alt="Docker" /></a>
  <a href="https://crates.io/crates/logdive"><img src="https://img.shields.io/crates/v/logdive.svg" alt="Crates.io" /></a>
  <a href="https://docs.rs/logdive-core"><img src="https://img.shields.io/docsrs/logdive-core" alt="Docs.rs" /></a>
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
</p>

# logdive

**Fast, self-hosted query engine for structured JSON logs.**

A single Rust binary that ingests structured logs, indexes them locally in SQLite, and lets you query them instantly from the CLI or an HTTP API. No infrastructure, no daemons, no cloud.

```bash
# Ingest JSON, logfmt, or plain-text logs from a file or stdin.
logdive ingest --file ./logs/app.log
logdive ingest --file ./logs/app.log --format logfmt --tag production
docker logs my-container | logdive ingest --tag my-container

# Tail a growing log file in real time (Ctrl-C to stop).
logdive ingest --file ./logs/app.log --follow

# Query with AND and OR.
logdive query 'level=error AND service=payments last 2h'
logdive query 'level=error OR level=warn' --output json

# Prune old entries to keep the index lean.
logdive prune --older-than 30d

# Inspect the index.
logdive stats

# Expose a read-only HTTP API for remote querying.
logdive-api --db ./logdive.db --port 4000
curl 'http://127.0.0.1:4000/query?q=level%3Derror&limit=100'
curl 'http://127.0.0.1:4000/version'
```

> **Status: v0.3.0.** Adds parenthesised query groups, CLI pagination (`--offset`), case-insensitive level queries, and a distroless Docker runtime. Breaking: `logdive query --format` renamed to `--output`; `execute()` now takes `QueryOptions { limit, offset }`. See [v1 non-goals](#v1-non-goals) for what is explicitly out of scope.

---

## Table of contents

- [Why logdive](#why-logdive)
- [Install](#install)
- [Quick start](#quick-start)
- [The `logdive` CLI](#the-logdive-cli)
- [Running with Docker](#running-with-docker)
- [The `logdive-api` HTTP server](#the-logdive-api-http-server)
- [Query language reference](#query-language-reference)
- [Configuration reference](#configuration-reference)
- [Architecture](#architecture)
- [Performance](#performance)
- [Development](#development)
- [v1 non-goals](#v1-non-goals)
- [License](#license)

---

## Why logdive

Every backend engineer has hit the same wall: the app is producing JSON logs, something went wrong in production, and the options are `grep`, an unreadable chain of `jq` pipes, or spinning up a full observability stack (Loki, Datadog, Elastic) that requires infrastructure, cost, and configuration you don't have time for in a side project or small team.

logdive sits in the gap. It's a single binary you drop anywhere. Point it at a log file — or pipe Docker output into it — and you get a fast, queryable index on your local machine. You can ask it `level=error AND service=payments last 2h` and get results in milliseconds. You can expose a lightweight HTTP endpoint so a minimal UI or a curl script can query it remotely.

The target user is a backend engineer who wants `jq` with memory, filters, and time ranges — without YAML files, without a running daemon they didn't ask for, without a monthly bill.

---

## Install

logdive ships two binaries: `logdive` (CLI) and `logdive-api` (HTTP server). They share a database format; you can ingest with the CLI and serve queries over HTTP, or vice versa.

### From crates.io

```bash
cargo install logdive logdive-api
```

Both binaries land in `~/.cargo/bin/` — make sure it's on your `PATH`.

### From Docker

Official multi-arch images (linux/amd64 and linux/arm64) are available on GHCR:

```bash
docker pull ghcr.io/aryagorjipour/logdive:latest
```

See [Running with Docker](#running-with-docker) for usage.

### From prebuilt binaries

Download the latest release for your platform from the [GitHub Releases](https://github.com/Aryagorjipour/logdive/releases) page. Binaries are currently built for:

- Linux x86_64
- macOS arm64

Extract the archive and move the binaries to any directory on your `PATH`.

### From source

```bash
git clone https://github.com/Aryagorjipour/logdive
cd logdive
cargo build --release
```

The compiled binaries will be at `target/release/logdive` and `target/release/logdive-api`.

MSRV: Rust 1.85 (edition 2024).

---

## Quick start

The `examples/` directory ships with two sample log files. Let's ingest them and run a few queries.

```bash
# Ingest both sample files into a throwaway database.
logdive --db /tmp/demo.db ingest --file examples/app.log
logdive --db /tmp/demo.db ingest --file examples/nginx.log

# See what we've got.
logdive --db /tmp/demo.db stats

# Find every error across both files.
logdive --db /tmp/demo.db query 'level=error'

# Find errors or warnings from a specific service.
logdive --db /tmp/demo.db query 'level=error OR level=warn AND service=payments'

# Find slow nginx requests.
logdive --db /tmp/demo.db query 'tag=nginx AND request_time > 1.0'

# Get structured output for further processing.
logdive --db /tmp/demo.db query 'service=payments' --output json | jq

# Prune entries older than 7 days.
logdive --db /tmp/demo.db prune --older-than 7d
```

See [`examples/README.md`](examples/README.md) for a longer walkthrough of what these files contain and what queries are interesting against them.

---

## The `logdive` CLI

Four subcommands: `ingest`, `query`, `stats`, `prune`.

### `logdive ingest`

Reads log lines from a file or stdin, parses them, and inserts them into the index.

```bash
# JSON (default)
logdive ingest --file ./logs/app.log
logdive ingest --file ./logs/app.log --tag production

# logfmt
logdive ingest --file ./logs/app.log --format logfmt

# Plain text (whole line becomes `message`)
logdive ingest --file ./logs/app.log --format plain

# Pipe from any source
docker logs my-container | logdive ingest --tag my-container
journalctl --output=json | logdive ingest --tag systemd

# Tail a growing file in real time
logdive ingest --file ./logs/app.log --follow
```

Flags:

- `--file <PATH>` / `-f` — Read from a file. Mutually exclusive with stdin.
- `--format json|logfmt|plain` — Input format. Default `json`.
- `--tag <TAG>` / `-t` — Attach a tag to every ingested entry that does not already contain a `tag` field.
- `--timestamp-now` — Assign the current UTC time (RFC 3339) to entries that lack a `timestamp` field, instead of skipping them. Useful for formats that do not include timestamps.
- `--follow` — Keep the file open and ingest new lines as they are appended, similar to `tail -f`. Detects log rotation (inode change) and truncation and reopens the file automatically. Ctrl-C exits cleanly. Requires `--file`.
- `--db <PATH>` — Override the default `~/.logdive/index.db` location (global, applies to all subcommands). Also settable via `$LOGDIVE_DB`.

Behavior:

- **Deduplication**: Every row is fingerprinted with a blake3 hash. Re-ingesting the same file (or a log rotation producing overlapping lines) results in zero duplicate rows.
- **Graceful skip**: Lines that cannot be parsed in the selected format are counted and skipped, not fatal. Blank lines are silently ignored.
- **No-timestamp skip**: By default, lines without a `timestamp` field are skipped. Pass `--timestamp-now` to assign the current UTC time to such entries instead.
- **Progress**: TTY-aware status on stderr. A final summary always prints inserted / deduplicated / skipped counts.

### `logdive query`

Runs a query against the index and renders matching entries.

```bash
logdive query 'level=error'
logdive query 'level=error AND service=payments last 24h'
logdive query 'level=error OR level=warn'
logdive query '(level=error OR level=warn) AND service=payments'
logdive query 'message contains "timeout"' --output json
logdive query 'since 2026-01-01' --limit 0
```

Flags:

- `--output pretty|json` — Output format. Default `pretty` (colored, human-readable). `json` is newline-delimited, pipe-friendly for `jq`.
- `--limit <N>` — Maximum results to return. Default `1000`. Use `0` for unlimited.
- `--offset <N>` — Skip the first N results. Use with `--limit` for page navigation. Default `0`.
- `--db <PATH>` — Database path override. Also settable via `$LOGDIVE_DB`.

Pretty output honors `NO_COLOR` and auto-strips ANSI when piped. JSON output is identical in shape to the HTTP API's `/query` response.

See the [Query language reference](#query-language-reference) for the full grammar and operator list.

### `logdive stats`

Reports aggregate metadata about the index.

```bash
logdive stats
```

Sample output:

```
logdive index: /home/user/.logdive/index.db
  Entries:       42,317
  Time range:    2026-03-14T08:22:01Z → 2026-04-22T19:45:03Z
  Tags:          api, nginx, payments, worker, (untagged)
  DB size:       8.4 MB (8,400,000 bytes)
```

