#!/usr/bin/env sh
# Pre-release verification for logdive.
#
# Runs the full battery of checks that must pass before cutting a
# release tag. Intended to be run locally by the release manager; the
# CI workflow already runs the equivalent of most of these on every
# push, but this script adds a few publish-specific checks and gives
# a single pass/fail summary for release readiness.
#
# Exit code 0 means "safe to tag and publish v$VERSION". Anything
# non-zero means at least one check failed and the output above
# explains which.

set -eu

# ---------------------------------------------------------------------
# Discover the workspace version from the root Cargo.toml.
# ---------------------------------------------------------------------

if [ ! -f "Cargo.toml" ]; then
  echo "error: run this script from the repo root (no Cargo.toml here)" >&2
  exit 1
fi

# Read `version = "X.Y.Z"` from [workspace.package]. Grep is enough —
# the manifest is ours and we control its shape.
VERSION=$(grep -E '^version\s*=\s*"[^"]+"' Cargo.toml | head -1 | sed -E 's/^version\s*=\s*"([^"]+)"/\1/')

if [ -z "$VERSION" ]; then
  echo "error: could not read workspace version from Cargo.toml" >&2
  exit 1
fi

echo "=============================================================="
echo "logdive pre-release check"
echo "=============================================================="
echo "Workspace version: $VERSION"
echo "Proposed tag:      v$VERSION"
echo ""

# ---------------------------------------------------------------------
# Helpers.
# ---------------------------------------------------------------------

step() {
  echo ""
  echo "--------------------------------------------------------------"
  echo "[step] $1"
  echo "--------------------------------------------------------------"
}

fail() {
  echo ""
  echo "=============================================================="
  echo "FAILED: $1"
  echo "=============================================================="
  exit 1
}

# ---------------------------------------------------------------------
# 1. Clean working tree.
# ---------------------------------------------------------------------

step "Verifying clean working tree"
if [ -n "$(git status --porcelain)" ]; then
  git status --short
  fail "Working tree has uncommitted changes. Commit or stash first."
fi
echo "OK: working tree is clean."

# ---------------------------------------------------------------------
# 2. On main branch.
# ---------------------------------------------------------------------

step "Verifying branch"
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
