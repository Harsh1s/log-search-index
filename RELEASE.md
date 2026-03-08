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

