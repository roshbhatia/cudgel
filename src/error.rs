//! Error types for cudgel

use thiserror::Error;

/// Error types for cudgel operations
///
/// All errors implement `std::error::Error` via thiserror and can be
/// converted using the `?` operator for ergonomic error propagation.
#[derive(Error, Debug)]
pub enum Error {
    /// Database query or connection error
    #[error("Database error: {0}\n\nTroubleshooting:\n  - Ensure PostgreSQL is running: ./scripts/start-postgres.sh\n  - Check database status: psql -h localhost -p 54321 -U cudgel -d cudgel -c 'SELECT 1'\n  - Verify pgvector extension: psql -h localhost -p 54321 -U cudgel -d cudgel -c 'SELECT * FROM pg_extension'")]
    Database(#[from] tokio_postgres::Error),

    /// Connection pool error
    #[error("Database connection pool error: {0}\n\nThis usually means:\n  - PostgreSQL is not running or not accessible\n  - Database connection limit reached\n\nTry:\n  - Restart PostgreSQL: ./scripts/stop-postgres.sh && ./scripts/start-postgres.sh\n  - Check PostgreSQL logs: tail -f ~/.local/share/cudgel/postgres.log")]
    Pool(#[from] deadpool_postgres::PoolError),

    /// Connection pool creation error
    #[error("Failed to create database connection pool: {0}\n\nSuggestion: Check your database configuration and ensure PostgreSQL 17 with pgvector is installed")]
    PoolCreation(String),

    /// File system I/O error
    #[error("File system error: {0}\n\nCheck that:\n  - The file/directory exists and is readable\n  - You have necessary permissions\n  - The path is correct")]
    Io(#[from] std::io::Error),

    /// Directory traversal error
    #[error("Error walking directory: {0}\n\nPossible causes:\n  - Permission denied for some files\n  - Broken symbolic links\n  - Files deleted during traversal")]
    WalkDir(#[from] walkdir::Error),

    /// Code parsing error (tree-sitter)
    #[error("Failed to parse code: {0}\n\nThis may indicate:\n  - Syntax errors in the source file\n  - Unsupported language features\n  - File encoding issues")]
    Parse(String),

    /// Embedding generation error
    #[error("Embedding generation failed: {0}\n\nTroubleshooting:\n  - Ensure ONNX model is downloaded to ./models/all-MiniLM-L6-v2/\n  - Download with: uv venv .venv && source .venv/bin/activate && uv pip install 'optimum[onnxruntime]' && python -c 'from optimum.onnxruntime import ORTModelForFeatureExtraction; from transformers import AutoTokenizer; model = ORTModelForFeatureExtraction.from_pretrained(\"sentence-transformers/all-MiniLM-L6-v2\", export=True); tokenizer = AutoTokenizer.from_pretrained(\"sentence-transformers/all-MiniLM-L6-v2\"); model.save_pretrained(\"./models/all-MiniLM-L6-v2\"); tokenizer.save_pretrained(\"./models/all-MiniLM-L6-v2\")'")]
    Embedding(String),

    /// ONNX Runtime error
    #[error("ONNX Runtime error: {0}")]
    OnnxRuntime(#[from] ort::Error),

    /// Configuration error
    #[error("Configuration error: {0}\n\nCudgel uses hardcoded local defaults. If you're seeing this, please file a bug report.")]
    Config(String),

    /// Unsupported programming language
    #[error("Language '{0}' is not supported\n\nSupported languages:\n  - Python (.py, .pyw)\n  - JavaScript (.js, .jsx, .mjs)\n  - TypeScript (.ts, .tsx)\n  - Rust (.rs)\n  - Go (.go)\n  - C (.c, .h)\n  - C++ (.cpp, .cc, .cxx, .hpp, .hh, .hxx)\n  - Java (.java)")]
    UnsupportedLanguage(String),

    /// Symbol lookup failed
    #[error("Symbol '{0}' not found in indexed repositories\n\nTry:\n  - Run 'cudgel index /path/to/repo' to index the codebase first\n  - Use 'cudgel query \"{0}\"' for fuzzy search\n  - Check spelling and case sensitivity")]
    SymbolNotFound(String),

    /// Repository not found
    #[error("Repository not found: {0}\n\nSuggestion: Index the repository first with 'cudgel index {0}'")]
    RepositoryNotFound(String),

    /// Database schema not initialized
    #[error("Database schema not initialized\n\nRun: cudgel init-db\n\nThis will create all required tables and indexes.")]
    SchemaNotInitialized,

    /// PostgreSQL not running
    #[error("PostgreSQL is not running or not responding on port 54321\n\nStart PostgreSQL with: ./scripts/start-postgres.sh\n\nCheck status: lsof -i :54321")]
    PostgresNotRunning,

    /// Generic error for all other cases
    #[error("{0}")]
    Other(String),
}

/// Result type alias using cudgel's Error type
///
/// Used throughout the codebase for consistent error handling.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Check if error is related to PostgreSQL not running
    pub fn is_connection_refused(&self) -> bool {
        match self {
            Error::Database(e) => e.to_string().contains("Connection refused")
                || e.to_string().contains("connection refused"),
            Error::Pool(e) => e.to_string().contains("Connection refused")
                || e.to_string().contains("connection refused"),
            _ => false,
        }
    }

    /// Check if error is related to missing pgvector extension
    pub fn is_missing_pgvector(&self) -> bool {
        match self {
            Error::Database(e) => {
                e.to_string().contains("extension \"vector\"")
                    || e.to_string().contains("type \"vector\" does not exist")
            }
            _ => false,
        }
    }

    /// Check if error is related to missing schema
    pub fn is_missing_schema(&self) -> bool {
        match self {
            Error::Database(e) => {
                e.to_string().contains("relation")
                    && (e.to_string().contains("does not exist")
                        || e.to_string().contains("not found"))
            }
            _ => false,
        }
    }

    /// Convert to a more specific error with better context
    pub fn with_context(self) -> Self {
        if self.is_connection_refused() {
            return Error::PostgresNotRunning;
        }

        if self.is_missing_schema() {
            return Error::SchemaNotInitialized;
        }

        self
    }
}
