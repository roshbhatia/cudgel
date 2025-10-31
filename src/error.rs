//! Error types for cudgel

use thiserror::Error;

/// Error types for cudgel operations
///
/// All errors implement `std::error::Error` via thiserror and can be
/// converted using the `?` operator for ergonomic error propagation.
#[derive(Error, Debug)]
pub enum Error {
    /// Database query or connection error
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    /// Connection pool error
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    /// Connection pool creation error
    #[error("Pool creation error: {0}")]
    PoolCreation(String),

    /// File system I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Directory traversal error
    #[error("WalkDir error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Code parsing error (tree-sitter)
    #[error("Parse error: {0}")]
    Parse(String),

    /// Embedding generation error
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Unsupported programming language
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),

    /// Symbol lookup failed
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Generic error for all other cases
    #[error("{0}")]
    Other(String),
}

/// Result type alias using cudgel's Error type
///
/// Used throughout the codebase for consistent error handling.
pub type Result<T> = std::result::Result<T, Error>;
