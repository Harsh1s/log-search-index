# logdive examples

Sample log files and a query cookbook covering every feature of logdive v0.3.0.

## Files

| File | Format | What it is | Entries |
|---|---|---|---|
| `app.log` | JSON | Backend services: `payments`, `orders`, `auth` | 60 |
| `nginx.log` | JSON | Nginx access logs | 25 |
| `worker.log` | logfmt | Background job worker: email and notification queues | 41 |
| `deploy.log` | plain text | Deployment pipeline output (no structured fields) | 28 |

All four files share the same time window (2026-04-15, 09:00–12:51 UTC).

---

## Setup

These examples use a throwaway database so they don't touch your default index. Run from the repository root.

### Ingest all files

```bash
DB=/tmp/logdive-examples.db

# JSON files — default format
logdive --db $DB ingest --file examples/app.log
logdive --db $DB ingest --file examples/nginx.log

# logfmt file — specify format explicitly
logdive --db $DB ingest --file examples/worker.log --format logfmt

# Plain text — no structured fields; --timestamp-now stamps each line at ingest time
logdive --db $DB ingest --file examples/deploy.log --format plain --timestamp-now
```

### Check what you ingested

```bash
logdive --db $DB stats
```

Expected output:
```
entries:    154
time range: 2026-04-15T09:00:05Z → 2026-04-15T12:51:19Z
tags:       auth (24), nginx (25), orders (19), payments (18), worker (41), (untagged) (3 + 28 deploy lines)
db size:    ~0.3 MB
```

---

## Query cookbook

### Alias for brevity

```bash
alias ld='logdive --db /tmp/logdive-examples.db query'
```

---

### Basic level filtering

```bash
# All errors across every source
ld 'level=error'

# All warnings
ld 'level=warn'

# Case-insensitive level matching — new in v0.3.0
# These three return identical results
ld 'level=error'
ld 'level=ERROR'
ld 'level=Error'

# Exclude info noise — show everything that isn't info
ld 'level!=info'
```

---

### OR queries — new in v0.3.0

```bash
# Errors or warnings from any source
ld 'level=error OR level=warn'

# Worker queue errors or queue connection issues
ld 'queue=email OR queue=notifications'

# Payments or auth service entries
ld 'service=payments OR service=auth'
```

---

### Parenthesised groups — new in v0.3.0

Combine OR branches with AND conditions using parentheses.

```bash
# Errors or warnings, but only from payments
ld '(level=error OR level=warn) AND service=payments'

# Errors from payments or auth
ld '(service=payments OR service=auth) AND level=error'

# Failed jobs from either queue
ld '(queue=email OR queue=notifications) AND level=error'

# Auth problems: rate limiting or invalid tokens
ld '(message contains "rate limit" OR message contains "invalid token") AND tag=auth'

# nginx 4xx and 5xx responses from a specific IP range
ld '(status > 399) AND tag=nginx'

# Critical worker events — errors or permanent failures
ld '(level=error OR message contains "permanently") AND tag=worker'
```

---

### AND chains

```bash
# Slow payments (over 5 seconds)
ld 'service=payments AND duration_ms > 5000'

# Nginx 5xx responses
ld 'tag=nginx AND status > 499'

# Auth errors involving a specific user
ld 'service=auth AND level=error AND user_id=1147'

# Worker job failures with specific queue
ld 'tag=worker AND level=error AND queue=email'
```

---

### CONTAINS — full-text substring match

```bash
# Any mention of timeout
ld 'message contains "timeout"'

# Job failure events
ld 'message contains "failed"'

# Database-related events
ld 'message contains "database"'

# nginx scanner/probe traffic
ld 'tag=nginx AND message contains "access" AND path contains "wp-admin"'

# Payment errors mentioning specific error codes
ld 'message contains "GATEWAY_TIMEOUT" OR message contains "CARD_DECLINED"'
```

---

### Field comparisons

```bash
# Slow requests from nginx (over 1 second)
ld 'request_time > 1.0'

# Very fast responses only
ld 'request_time < 0.010'

# High-latency backend calls (over 2 seconds)
ld 'duration_ms > 2000'

# Large orders
ld 'total > 100'

# Specific HTTP status
ld 'status=503'

# Worker jobs that took over 300ms
ld 'tag=worker AND duration_ms > 300'
```

---

### Time-range queries

```bash
# Everything from 11:00 UTC onwards
ld 'since 2026-04-15T11:00:00Z'

# Errors in the morning window only
ld 'level=error AND since 2026-04-15T09:00:00Z'

# Combine with OR groups and time range
ld '(level=error OR level=warn) AND service=payments AND since 2026-04-15T09:20:00Z'

# Note: 'last Nh/Nd' is relative to now — use 'since' for fixture data with fixed timestamps
# For live logs: ld 'level=error last 1h'
```

---

### Tag-based filtering

```bash
# All nginx access log entries
ld 'tag=nginx'

# Errors from any tagged source
ld 'level=error AND tag=payments'

# Worker queue stats only
ld 'tag=worker AND message contains "queue stats"'

# Everything not from nginx
ld 'level=error AND tag!=nginx'
```

---

### Output formats — `--output` flag, new in v0.3.0

`--output` was renamed from `--format` in v0.3.0.

```bash
# Pretty-printed output (default) — human readable with colours
ld 'level=error' --output pretty

# JSON output — one JSON object per line (NDJSON), pipe-friendly
ld 'level=error' --output json

# Pipe to jq — extract specific fields
ld 'level=error' --output json | jq '{ts: .timestamp, svc: .service, msg: .message}'

# Aggregate: count errors per service
ld 'level=error' --output json | jq -r '.service' | sort | uniq -c | sort -rn

# Extract all error codes from payments
ld 'service=payments AND level=error' --output json | jq '.error_code // empty'

# Flatten all fields from worker failures
ld 'tag=worker AND level=error' --output json | jq '.'

# Build a CSV of slow nginx requests
ld 'tag=nginx AND request_time > 0.5' --output json \
  | jq -r '[.timestamp, .path, .status, .request_time] | @csv'
```

---

### Pagination — `--limit` and `--offset`, new in v0.3.0

```bash
# First 5 errors
ld 'level=error' --limit 5

# Next 5 (page 2)
ld 'level=error' --limit 5 --offset 5

# Page 3
ld 'level=error' --limit 5 --offset 10

# Unlimited results (override default limit of 1000)
ld 'tag=worker' --limit 0

# Combine pagination with JSON output for scripted consumption
ld 'level=error' --limit 10 --offset 0 --output json
ld 'level=error' --limit 10 --offset 10 --output json
```

---

### Realistic investigation workflows

**"What went wrong around 09:22?"**

```bash
ld 'since 2026-04-15T09:20:00Z' --limit 20
```

**"Show me the full payment failure chain for order ord_28472"**

```bash
ld 'order_id=ord_28472' --output json | jq '{ts: .timestamp, svc: .service, lvl: .level, msg: .message}'
```

**"Are there any errors or warnings I should care about right now?"**

```bash
ld 'level=error OR level=warn' --output pretty
```

**"Which user triggered rate limiting?"**

```bash
ld 'message contains "rate limit" OR message contains "rate_limit"' --output json | jq '.user_id // empty'
```

**"Show me all nginx responses that weren't 200"**

```bash
ld 'tag=nginx AND status!=200' --output json | jq '{ts: .timestamp, path: .path, status: .status}'
```

