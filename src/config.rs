//! Configuration management
//!
//! Hardcoded for local-only usage. No env vars required - everything "just works".

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub temporal: TemporalConfig,
    pub embedding: EmbeddingConfig,
    pub lsp: LspConfig,
    pub indexing: IndexingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    pub host: String,
    pub namespace: String,
    pub task_queue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model_path: PathBuf,
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub batch_size: usize,
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
