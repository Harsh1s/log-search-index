# traps.md

Known pitfalls. Each section is one trap.

---

## tower-http ↔ axum version coupling

`tower-http` and `axum` share the same `tower` trait objects and `http` types
internally. They must be version-compatible with each other. As of v0.3.0:
`axum = "0.7"` + `tower-http = "0.6"` + `tower = "0.4"`. Bumping any one of
these can break compilation with type-mismatch errors on `Service` impls or
`Body` types. When bumping axum, always check tower-http's changelog for the
required matching version. Note that `tower-http` is declared directly in
`crates/api/Cargo.toml` (not inherited from workspace), so a workspace-level
axum bump must be accompanied by a manual tower-http bump in api's Cargo.toml.
As of v0.3.0: `axum = "0.7"` + `tower-http = "0.6"` + `tower = "0.5"`.
(tower was bumped from 0.4 → 0.5 in v0.3.0 to unify with axum's transitive dep.)

---

## cargo publish dependency order (core → cli → api, ~30s waits)

Cargo resolves `path = "..."` workspace dependencies as registry lookups when
packaging. Publishing `logdive` or `logdive-api` before `logdive-core` is live
on crates.io will fail with "no matching package named logdive-core". Always
publish in order: `logdive-core` → wait ~30 seconds for index propagation →
`logdive` → wait ~30 seconds → `logdive-api`. The prerelease script reminds you
but does not automate the waits.

---

## GHA cache mode=max causes 502s on Docker workflow

The Docker workflow (`docker.yml`) writes the BuildKit layer cache back to the
GHA cache backend with `mode=min` (final-image layers only). Using `mode=max`
(all intermediate layers) produces a much larger export artifact that
reliably hits transient 502 errors from the GHA cache backend during the write
phase, failing CI for no build-related reason. Always use `mode=min` for the
write leg. PR builds use read-only cache (never write) for the same reason.

---

## SQLite VACUUM cannot run inside an explicit transaction

`Indexer::prune` issues DELETE and VACUUM as two separate autocommit statements
rather than wrapping them in `conn.transaction()`. SQLite rejects VACUUM inside
an explicit transaction with "cannot VACUUM from within a transaction." A crash
between the DELETE and VACUUM leaves rows deleted but file size not reclaimed —
harmless, since any later VACUUM reclaims the space. Do not wrap prune logic in
a single transaction.

---

## LOGDIVE_API_HOST must be 0.0.0.0 in Docker containers

The API server defaults to binding `127.0.0.1` (loopback only). Inside a Docker
container this means the server is unreachable from `-p 4000:4000` port
forwarding — the port is bound but only accessible from inside the container's
network namespace. The Dockerfile sets `ENV LOGDIVE_API_HOST=0.0.0.0` to
override this for container deployments. If you add a new deployment target
(Fly.io config, docker-compose.yml, etc.) that doesn't inherit the Dockerfile's
ENV, you must set this explicitly or the server will appear to start but be
unreachable.

---

## ensure_index_exists() required for container first-run

On a fresh Docker volume, the `/data` directory exists but is empty.
`logdive-api` calls `ensure_index_exists()` at startup to create the schema
before handing off to `AppState::with_connection` (which calls `open_read_only`
— if the file didn't exist, this would immediately fail every request with a
SQLite "unable to open" error). Any code path that opens the DB read-only must
assume the file already exists. If you're adding a new startup flow, call
`ensure_index_exists` or `Indexer::open` first.

---

## MSRV 1.85 — do not use post-1.85 features

The workspace sets `rust-version = "1.85"` and CI runs a dedicated `msrv` job
that builds on exactly 1.85. Features stabilized after 1.85 will fail this job.
When adopting a new language feature, verify it was stable in 1.85 by checking
the Rust release notes.

---

## Binary name is logdive (not cli), crate path is crates/cli/

The CLI's crate name (`name = "logdive"` in Cargo.toml), binary name
(`[[bin]] name = "logdive"`), and the name used in `cargo run --bin logdive`
are all `logdive`. The crate lives at `crates/cli/` but that path is an
implementation detail. Never refer to it as "the cli crate" in user-visible
contexts, commit messages, or docs. `cargo run --bin cli` will fail.

---

## cargo publish --workspace --dry-run requires Cargo 1.90+

The prerelease script detects the Cargo version and uses `cargo publish
--dry-run --workspace` only when Cargo ≥1.90. On older toolchains it falls back
to verifying only `logdive-core`. The full workspace dry-run is the authoritative
check; if you're on a toolchain older than 1.90, the pass on dry-run does not
