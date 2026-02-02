#!/usr/bin/env sh
# Verify release binaries are under the 10MB target.
#
# Usage: scripts/check-binary-size.sh [RELEASE_DIR]
#
# RELEASE_DIR defaults to `target/release`. For cross-compiled builds, pass
# the target-specific path, e.g. `target/aarch64-apple-darwin/release`.
#
# Exits 0 if both binaries are under the limit, 1 otherwise. Prints a
# human-readable table regardless of outcome so the output is useful
# in CI logs and during local development.

set -eu

RELEASE_DIR="${1:-target/release}"

# Maximum allowed size per binary, in bytes. 10 MB base-10 matches the
# project doc's Phase 4 target ("under 10MB") and what users see in `ls -h`.
MAX_BYTES=10000000

if [ ! -d "$RELEASE_DIR" ]; then
  echo "error: release directory not found: $RELEASE_DIR" >&2
  echo "hint: run \`cargo build --release\` first" >&2
  exit 1
fi

# BINARIES is whitespace-separated, not newline-separated, so it's
