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

    #[error("PostgreSQL not running on port 45678. Run: cudgel deps")]
    PostgresNotRunning,

    #[error("pgvector extension not installed. Run: cudgel deps")]
    PgvectorNotInstalled,

    // Dependency management errors
    #[error("Dependency missing: {0}")]
    DependencyMissing(String),

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    #[error("Database connection failed: {0}")]
    DatabaseConnectionFailed(String),

    #[error("Model download failed: {0}")]
    ModelDownloadFailed(String),

    #[error("Database start failed: {0}")]
    DatabaseStartFailed(String),

    #[error("Database stop failed: {0}")]
    DatabaseStopFailed(String),

    #[error("Schema initialization failed: {0}")]
    SchemaInitFailed(String),

    #[error("Insufficient disk space: required {required} MB, available {available} MB")]
    InsufficientDiskSpace { required: u64, available: u64 },

    #[error("Corrupted model: {0}")]
    CorruptedModel(String),

    #[error("Invalid dependency state: {0}")]
    InvalidDependencyState(String),

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

    #[error("Invalid tokenization strategy '{0}'. Valid options: 'onnx', 'fallback'\n\nSet via environment variable:\n  export CUDGEL_TOKENIZER_STRATEGY=fallback\n\nStrategy details:\n  • 'onnx' (default): Best quality, requires model download (cudgel deps)\n  • 'fallback': Offline mode, no downloads required, reduced quality")]
    InvalidTokenizerStrategy(String),

    #[error("{0}")]
    Other(String),

    #[error("Knowledge graph error: {0}")]
    Kg(#[from] crate::kg::KgError),
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
                    "{}\n\nRun: cudgel deps\n\nThis will automatically start PostgreSQL and initialize the database.",
                    self
                )
            }
            Error::PgvectorNotInstalled => {
                format!(
                    "{}\n\nRun: cudgel deps\n\nThis will install the pgvector extension automatically.",
                    self
                )
            }
            Error::SchemaNotInitialized => {
                format!(
                    "{}\n\nRun: cudgel deps\n\nThis will initialize the database schema with all required tables and indexes.",
                    self
                )
            }
            Error::DependencyMissing(dep) => {
                format!(
                    "Dependency missing: {}\n\nRun: cudgel deps\n\nThis will install all required dependencies automatically.",
                    dep
                )
            }
            Error::ModelDownloadFailed(msg) => {
                format!(
                    "Model download failed: {}\n\nTroubleshooting:\n  1. Check internet connection\n  2. Verify disk space (need ~500MB free)\n  3. Retry: cudgel deps",
                    msg
                )
            }
            Error::DatabaseStartFailed(msg) => {
                format!(
                    "Database start failed: {}\n\nTroubleshooting:\n  1. Check if port 45678 is available: lsof -i :45678\n  2. Check PostgreSQL logs: tail ~/.local/state/cudgel/postgres.log\n  3. Retry: cudgel deps",
                    msg
                )
            }
            Error::DatabaseStopFailed(msg) => {
                format!(
                    "Database stop failed: {}\n\nTroubleshooting:\n  1. Check process: ps aux | grep postgres\n  2. Force stop if needed: pkill -9 postgres",
                    msg
                )
            }
            Error::SchemaInitFailed(msg) => {
                format!(
                    "Schema initialization failed: {}\n\nTroubleshooting:\n  1. Ensure database is running: cudgel deps --check\n  2. Check PostgreSQL version (need 15+): postgres --version\n  3. Retry: cudgel deps",
                    msg
                )
            }
            Error::InsufficientDiskSpace { required, available } => {
                format!(
                    "Insufficient disk space\n\nRequired: {} MB\nAvailable: {} MB\n\nFree up space and retry: cudgel deps",
                    required, available
                )
            }
            Error::CorruptedModel(msg) => {
                format!(
                    "Corrupted model: {}\n\nTo fix:\n  1. Remove corrupted files: cudgel deps --clean\n  2. Re-download: cudgel deps",
                    msg
                )
            }
            Error::InvalidDependencyState(msg) => {
                format!(
                    "Invalid dependency state: {}\n\nTry:\n  cudgel deps --check --verbose\n\nFor full diagnostics.",
                    msg
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
