//! Configuration management
//!
//! Hardcoded for local-only usage. No env vars required - everything "just works".

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for the Cudgel code indexing tool
///
/// Contains all subsystem configurations with hardcoded local defaults.
/// No environment variables required - everything works out of the box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Database connection configuration
    pub database: DatabaseConfig,
    /// Temporal workflow engine configuration
    pub temporal: TemporalConfig,
    /// Embedding model configuration for semantic search
    pub embedding: EmbeddingConfig,
    /// Language Server Protocol configuration
    pub lsp: LspConfig,
    /// Code indexing behavior configuration
    pub indexing: IndexingConfig,
}

/// PostgreSQL database configuration
///
/// Uses non-standard port 54321 to avoid conflicts with system PostgreSQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database server hostname (default: localhost)
    pub host: String,
    /// Database server port (default: 54321)
    pub port: u16,
    /// Database name (default: cudgel)
    pub database: String,
    /// Database user (default: current system user)
    pub user: String,
    /// Database password (default: cudgel)
    pub password: String,
}

/// Temporal workflow engine configuration
///
/// Used for scheduled periodic re-indexing workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    /// Temporal server address (default: localhost:7234)
    pub host: String,
    /// Temporal namespace (default: default)
    pub namespace: String,
    /// Task queue name for indexing workflows (default: cudgel-indexing)
    pub task_queue: String,
}

/// Embedding model configuration for semantic code search
///
/// Currently uses dummy embeddings; production requires ONNX model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Path to ONNX model directory (default: ./models/all-MiniLM-L6-v2)
    pub model_path: PathBuf,
    /// Embedding vector dimension (default: 384)
    pub dimension: usize,
}

/// Language Server Protocol configuration
///
/// Controls LSP server behavior for IDE integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// LSP server port (default: 6010)
    pub port: u16,
}

/// Code indexing behavior configuration
///
/// Controls how files are processed during indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Number of files to process in one database batch (default: 100)
    pub batch_size: usize,
    /// Maximum file size to index in bytes (default: 1MB)
    pub max_file_size: usize,
}

impl Config {
    /// Create a new config with hardcoded local defaults
    /// Uses non-standard ports to avoid conflicts (PostgreSQL: 54321, Temporal: 7234)
    pub fn local() -> Self {
        Config {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 54321,
                database: "cudgel".to_string(),
                user: std::env::var("USER").unwrap_or_else(|_| "cudgel".to_string()),
                password: "cudgel".to_string(),
            },
            temporal: TemporalConfig {
                host: "localhost:7234".to_string(),
                namespace: "default".to_string(),
                task_queue: "cudgel-indexing".to_string(),
            },
            embedding: EmbeddingConfig {
                model_path: PathBuf::from("./models/all-MiniLM-L6-v2"),
                dimension: 384,
            },
            lsp: LspConfig { port: 6010 },
            indexing: IndexingConfig {
                batch_size: 100,
                max_file_size: 1024 * 1024,
            },
        }
    }

    /// Backward compatibility - just returns local config
    pub fn from_env() -> crate::Result<Self> {
        Ok(Self::local())
    }

    /// Generate PostgreSQL connection string
    ///
    /// Returns a connection string in libpq format for use with tokio-postgres.
    pub fn database_url(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password={}",
            self.database.host,
            self.database.port,
            self.database.database,
            self.database.user,
            self.database.password
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::local()
    }
}
