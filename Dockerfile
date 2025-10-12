# syntax=docker/dockerfile:1
#
# Multi-stage build for logdive.
#
# Stages:
#   chef     – current stable Rust toolchain with cargo-chef installed (shared base)
#   planner  – computes the dependency recipe from manifests + lockfile
#   builder  – cooks dependencies (cacheable layer), then builds binaries
#   runtime  – distroless/cc-debian12:nonroot — no shell, no tools, no root
#
# Both binaries are shipped in one image:
#   - logdive-api  (default ENTRYPOINT — HTTP server)
#   - logdive      (CLI — invoke via: docker run --entrypoint logdive ...)
#
# Data volume: mount a named volume at /data for index persistence.
#   docker run -v logdive-data:/data -p 4000:4000 ghcr.io/aryagorjipour/logdive

# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — chef
# Current stable Rust (always >= project MSRV of 1.85) with cargo-chef.
# Using rust:1 rather than rust:1.85 because cargo-chef's own dependencies
# require a newer compiler than the project MSRV. MSRV is enforced by CI
# (cargo check / cargo test), not by the Docker builder.
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1 AS chef
RUN cargo install cargo-chef --locked
WORKDIR /build

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — planner
# Reads every Cargo.toml in the workspace plus Cargo.lock and emits a
# recipe.json describing exactly which dependencies need to be compiled.
# This stage is re-run only when the dependency graph changes.
# ─────────────────────────────────────────────────────────────────────────────
FROM chef AS planner
COPY . .
