//! Format-aware line parsing for log ingestion.
//!
//! v0.2.0 introduced multiple input formats. This module contains:
//!   - the per-format parsers as submodules (`json`, `logfmt`, `plain`),
//!   - the [`LogFormat`] enum used to select among them,
//!   - and the [`parse_line`] dispatcher that routes each line to the
//!     right submodule based on the chosen format.
//!
//! All three submodules expose a `parse_line(line: &str) -> Option<LogEntry>`
//! function with the same graceful-skip contract: returns `None` on empty,
//! whitespace-only, or unparseable input. The dispatcher is a thin
//! `match` over the format selector — power users who already know the
//! format ahead of time can call the submodule directly.
//!
//! Submodules are intentionally `pub mod`: third-party consumers (a
//! plugin doing a custom ingestion pipeline, say) sometimes want to bypass
//! the dispatcher. The dispatcher remains the canonical entry point and
//! is what the CLI ingest path uses.

pub mod json;
pub mod logfmt;
pub mod plain;

use crate::entry::LogEntry;

/// Selects which line parser the dispatcher uses.
///
/// `Default` is [`LogFormat::Json`] — the v0.1.0 default carried forward
/// so callers that don't explicitly pick a format get the same behavior
/// they used to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Structured JSON, one object per line. The v0.1.0 default.
    #[default]
    Json,
    /// logfmt — `key=value` pairs, see `parsers::logfmt`.
    Logfmt,
    /// Unstructured plaintext. The whole line becomes `LogEntry::message`.
    Plain,
}

impl LogFormat {
    /// Every [`LogFormat`] variant, in declaration order.
    ///
    /// Use this when you need to enumerate all supported formats without
    /// hard-coding the list at a call site — for example, the API
    /// `/version` endpoint reports this slice so clients know what the
    /// running binary accepts. Adding a new variant here automatically
    /// propagates to every such consumer.
    pub const ALL: &'static [Self] = &[Self::Json, Self::Logfmt, Self::Plain];

    /// Parse a CLI-style format name. Case-insensitive.
    ///
    /// Returns `None` for unrecognized names. The CLI wraps this in a
    /// `clap` value parser that surfaces the unknown name as a usage
    /// error; library consumers can call it directly.
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Some(Self::Json),
            "logfmt" => Some(Self::Logfmt),
            "plain" => Some(Self::Plain),
            _ => None,
        }
    }

    /// Canonical short name used in CLI flags, configuration, and any
    /// future `Display`-based contexts. Always one lowercase word that
    /// round-trips through [`Self::from_name`].
    pub fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
