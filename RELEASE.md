# Release runbook

Steps to cut a new logdive release. Targeted at the project maintainer.

## One-time setup

You'll need:

- Push access to the main branch of this repository.
- A [crates.io account](https://crates.io/) with a valid API token.
- Run `cargo login <your-token>` once per machine.

## Release procedure

### 1. Prepare the version

- Decide the new version number following [semver](https://semver.org/).
  - Breaking API change in `logdive-core` → major bump.
  - New feature, backwards-compatible → minor bump.
  - Bug fix only → patch bump.
- Update `version` in the workspace root `Cargo.toml` under `[workspace.package]`.
- Run `cargo build --workspace` so the lockfile updates.
- Update `CHANGELOG.md`:
  - Promote the `[Unreleased]` heading to `[X.Y.Z] - YYYY-MM-DD` (today's date).
  - Add a fresh empty `[Unreleased]` section above it.
  - Update the link references at the bottom.
- Commit the changes:

  ```bash
  git add -A
  git commit -m "release: prepare vX.Y.Z"
  git push origin main
  ```

### 2. Run pre-release checks

```bash
sh scripts/prerelease-check.sh
```

This runs the full battery: clean working tree, build, tests, clippy, fmt, binary size check, and `cargo publish --dry-run` for each of the three crates. **Do not proceed if anything fails.**

A note on the dry-run step: the script detects whether your Cargo supports `cargo publish --workspace` (stabilized in Cargo 1.90, September 2025). If so, it runs a single workspace-scoped dry-run that verifies all three crates together using an internal local-registry overlay — this is the full verification. If you're on an older Cargo, the script only verifies `logdive-core` (the library); `logdive` and `logdive-api` can't be fully dry-run-verified before `logdive-core` is published, but they'll be validated at real publish time (step 4 below).

### 3. Tag and push

```bash
VERSION=X.Y.Z
git tag -a v$VERSION -m "v$VERSION"
git push origin main v$VERSION
```

The tag push triggers `.github/workflows/release.yml`, which builds release binaries for Linux x86_64 and macOS arm64, packages them as tarballs, and uploads them to a new GitHub Release.

Watch the workflow run in the [Actions tab](https://github.com/Aryagorjipour/logdive/actions). Typical runtime: ~10 minutes.

### 4. Publish to crates.io

Publish in dependency order. Wait ~30 seconds between each step — the crates.io index needs time to propagate before the next crate can reference the newly published one.

```bash
cargo publish -p logdive-core
# wait ~30s
cargo publish -p logdive
# wait ~30s
cargo publish -p logdive-api
```

Verify each crate appears on its crates.io page before moving on:

- <https://crates.io/crates/logdive-core>
- <https://crates.io/crates/logdive>
