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
