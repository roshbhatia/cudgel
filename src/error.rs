use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    #[error("Connection pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    #[error("Failed to create connection pool: {0}")]
    PoolCreation(String),

    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory traversal error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Embedding generation failed: {0}")]
    Embedding(String),

    #[error("ONNX Runtime error: {0}")]
    OnnxRuntime(#[from] ort::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("Database schema not initialized")]
    SchemaNotInitialized,

    #[error("PostgreSQL not running on port 54321")]
    PostgresNotRunning,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn is_connection_refused(&self) -> bool {
        match self {
            Error::Database(e) => {
                let msg = e.to_string();
                msg.contains("Connection refused") || msg.contains("connection refused")
            }
            Error::Pool(e) => {
                let msg = e.to_string();
                msg.contains("Connection refused") || msg.contains("connection refused")
            }
            _ => false,
        }
    }

    pub fn is_missing_pgvector(&self) -> bool {
        match self {
            Error::Database(e) => {
                let msg = e.to_string();
                msg.contains("extension \"vector\"") || msg.contains("type \"vector\" does not exist")
            }
            _ => false,
        }
    }

    pub fn is_missing_schema(&self) -> bool {
        match self {
            Error::Database(e) => {
                let msg = e.to_string();
                msg.contains("relation") && (msg.contains("does not exist") || msg.contains("not found"))
            }
            _ => false,
        }
    }

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
