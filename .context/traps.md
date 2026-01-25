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

