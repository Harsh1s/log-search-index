//! Unified error type for `logdive-core`.
//!
//! All fallible APIs in this crate return [`Result<T>`], aliased to
//! `std::result::Result<T, LogdiveError>`. Module-local error types
//! from earlier milestones (e.g. [`QueryParseError`] from the `query`
//! module) are preserved as public types and convert into `LogdiveError`
//! via `From` impls, so `?` works seamlessly at API boundaries while
//! callers that need structured access (the CLI rendering parse-error
//! carets, for example) can still match against the richer inner type.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::query::QueryParseError;

/// Convenient crate-wide result alias.
pub type Result<T> = std::result::Result<T, LogdiveError>;

/// Every error the core crate can produce.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LogdiveError {
    /// Wrapper around a [`QueryParseError`], preserved as-is so callers
    /// that want the structured position/message can match on it.
    #[error(transparent)]
    QueryParse(#[from] QueryParseError),

    /// The `since <datetime>` clause contained a string that did not
    /// parse as any accepted datetime format.
    #[error("invalid datetime {input:?}: {reason}")]
    InvalidDatetime { input: String, reason: String },

    /// A field name slipped through the parser's validation. This is a
    /// defense-in-depth guard at the SQL-generation boundary and should
    /// be unreachable in practice.
    #[error("unsafe field name {0:?}")]
    UnsafeFieldName(String),

    /// A row came back from SQLite with a malformed `fields` JSON column.
    /// Indicates corruption or an out-of-band write to the database.
    #[error("corrupt fields JSON in row: {0}")]
    CorruptFieldsJson(#[source] serde_json::Error),

    /// Underlying SQLite error.
