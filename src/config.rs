//! Configuration management

use serde::{Deserialize, Serialize};
use std::env;
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
    pub fn from_env() -> crate::Result<Self> {
        dotenv::dotenv().ok();

        let port = Self::parse_env_u16("CUDGEL_DB_PORT", 5432)?;
        let embedding_dimension = Self::parse_env_usize("CUDGEL_EMBEDDING_DIMENSION", 384)?;
        let lsp_port = Self::parse_env_u16("CUDGEL_LSP_PORT", 6010)?;
        let batch_size = Self::parse_env_usize("CUDGEL_INDEX_BATCH_SIZE", 100)?;
        let max_file_size = Self::parse_env_usize("CUDGEL_MAX_FILE_SIZE", 1024 * 1024)?;

        Ok(Config {
            database: DatabaseConfig {
                host: env::var("CUDGEL_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port,
                database: env::var("CUDGEL_DB_NAME").unwrap_or_else(|_| "cudgel".to_string()),
                user: env::var("CUDGEL_DB_USER").unwrap_or_else(|_| "cudgel".to_string()),
                password: env::var("CUDGEL_DB_PASSWORD").unwrap_or_else(|_| "cudgel".to_string()),
            },
            temporal: TemporalConfig {
                host: env::var("CUDGEL_TEMPORAL_HOST")
                    .unwrap_or_else(|_| "localhost:7233".to_string()),
                namespace: env::var("CUDGEL_TEMPORAL_NAMESPACE")
                    .unwrap_or_else(|_| "default".to_string()),
                task_queue: env::var("CUDGEL_TEMPORAL_TASK_QUEUE")
                    .unwrap_or_else(|_| "cudgel-indexing".to_string()),
            },
            embedding: EmbeddingConfig {
                model_path: PathBuf::from(
                    env::var("CUDGEL_EMBEDDING_MODEL_PATH")
                        .unwrap_or_else(|_| "./models/all-MiniLM-L6-v2".to_string()),
                ),
                dimension: embedding_dimension,
            },
            lsp: LspConfig { port: lsp_port },
            indexing: IndexingConfig {
                batch_size,
                max_file_size,
            },
        })
    }

    fn parse_env_u16(key: &str, default: u16) -> crate::Result<u16> {
        match env::var(key) {
            Ok(val) => val.parse().map_err(|_| {
                crate::Error::Config(format!(
                    "Invalid value for {}: must be a valid port number",
                    key
                ))
            }),
            Err(_) => Ok(default),
        }
    }

    fn parse_env_usize(key: &str, default: usize) -> crate::Result<usize> {
        match env::var(key) {
            Ok(val) => val.parse().map_err(|_| {
                crate::Error::Config(format!(
                    "Invalid value for {}: must be a positive number",
                    key
                ))
            }),
            Err(_) => Ok(default),
        }
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
        Self::from_env().unwrap_or_else(|_| Config {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "cudgel".to_string(),
                user: "cudgel".to_string(),
                password: "cudgel".to_string(),
            },
            temporal: TemporalConfig {
                host: "localhost:7233".to_string(),
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
        })
    }
}
