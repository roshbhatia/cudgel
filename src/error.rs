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

    #[error("PostgreSQL not running on port 54321. Please start PostgreSQL using: task db-start")]
    PostgresNotRunning,

    #[error("pgvector extension not installed. Please install with: CREATE EXTENSION vector;")]
    PgvectorNotInstalled,

    #[error("Orchestrator already running with PID {0}")]
    OrchestratorAlreadyRunning(i32),

    #[error("Orchestrator not running")]
    OrchestratorNotRunning,

    #[error("Invalid PID file: {0}")]
    InvalidPidFile(String),

    #[error("Failed to acquire task lock (retry {0})")]
    TaskLockFailed(i32),

    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    #[error("Graceful shutdown timeout after {0} seconds")]
    ShutdownTimeout(u64),

    #[error("Signal handling error: {0}")]
    SignalHandler(String),

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
                msg.contains("extension \"vector\"")
                    || msg.contains("type \"vector\" does not exist")
            }
            _ => false,
        }
    }

    pub fn is_missing_schema(&self) -> bool {
        match self {
            Error::Database(e) => {
                let msg = e.to_string();
                msg.contains("relation")
                    && (msg.contains("does not exist") || msg.contains("not found"))
            }
            _ => false,
        }
    }

    pub fn with_context(self) -> Self {
        if self.is_connection_refused() {
            return Error::PostgresNotRunning;
        }
        if self.is_missing_pgvector() {
            return Error::PgvectorNotInstalled;
        }
        if self.is_missing_schema() {
            return Error::SchemaNotInitialized;
        }
        self
    }

    /// Convert error to user-friendly message with actionable suggestions
    pub fn to_user_message(&self) -> String {
        match self {
            Error::PostgresNotRunning => {
                format!(
                    "{}\n\nTroubleshooting steps:\n  1. Start PostgreSQL: task db-start\n  2. Check PostgreSQL status: task db-status\n  3. Verify port 54321 is not in use: lsof -i :54321",
                    self
                )
            }
            Error::PgvectorNotInstalled => {
                format!(
                    "{}\n\nTo install:\n  1. Connect to PostgreSQL: psql -h localhost -p 54321 -U {} cudgel\n  2. Run: CREATE EXTENSION vector;",
                    self,
                    std::env::var("USER").unwrap_or_else(|_| "cudgel".to_string())
                )
            }
            Error::SchemaNotInitialized => {
                format!(
                    "{}\n\nInitialize database schema:\n  cudgel init-db\n\nThis will create all required tables and indexes.",
                    self
                )
            }
            Error::RepositoryNotFound(path) => {
                format!(
                    "Repository not found: {}\n\nHave you indexed this repository? Try:\n  cudgel index {}",
                    path, path
                )
            }
            Error::UnsupportedLanguage(lang) => {
                format!(
                    "Unsupported language: {}\n\nSupported languages: python, javascript, typescript, rust, go, c, cpp, java\nFile will be skipped during indexing.",
                    lang
                )
            }
            Error::Embedding(msg) => {
                format!(
                    "Embedding generation failed: {}\n\nCheck that the ONNX model is properly installed:\n  ls -la ~/.local/share/cudgel/models/all-MiniLM-L6-v2/\n\nTo reinstall:\n  task setup",
                    msg
                )
            }
            Error::OrchestratorAlreadyRunning(pid) => {
                format!(
                    "Orchestrator is already running (PID: {})\n\nTo stop it:\n  cudgel orchestrator stop\n\nTo restart:\n  cudgel orchestrator restart\n\nTo view status:\n  cudgel orchestrator status",
                    pid
                )
            }
            Error::OrchestratorNotRunning => {
                "Orchestrator is not running\n\nTo start it:\n  cudgel orchestrator start\n\nTo view scheduled tasks:\n  cudgel schedule --list".to_string()
            }
            Error::InvalidPidFile(msg) => {
                format!(
                    "Invalid PID file: {}\n\nThe orchestrator may have crashed or been terminated improperly.\nTry removing the stale PID file:\n  rm ~/.local/state/cudgel/orchestrator.pid\n\nThen start the orchestrator:\n  cudgel orchestrator start",
                    msg
                )
            }
            Error::ShutdownTimeout(secs) => {
                format!(
                    "Graceful shutdown timeout after {} seconds\n\nThe orchestrator may have hung tasks.\nIf the problem persists, try force-stopping:\n  pkill -9 cudgel",
                    secs
                )
            }
            _ => self.to_string(),
        }
    }
}
