//! Configuration management
//!
//! Follows XDG Base Directory specification with sensible defaults.
//! Checks environment variables first, then falls back to XDG defaults.

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
    /// Embedding model configuration for semantic search
    pub embedding: EmbeddingConfig,
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
    /// Create a new config with XDG-compliant defaults
    /// Checks environment variables first, then falls back to XDG defaults
    /// Uses non-standard port to avoid conflicts (PostgreSQL: 54321)
    pub fn local() -> Self {
        Config {
            database: DatabaseConfig {
                host: std::env::var("PGHOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("PGPORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(54321),
                database: std::env::var("PGDATABASE").unwrap_or_else(|_| "cudgel".to_string()),
                user: std::env::var("PGUSER")
                    .or_else(|_| std::env::var("USER"))
                    .unwrap_or_else(|_| "cudgel".to_string()),
                password: std::env::var("PGPASSWORD").unwrap_or_else(|_| "cudgel".to_string()),
            },
            embedding: EmbeddingConfig {
                model_path: xdg_data_home().join("cudgel/models/all-MiniLM-L6-v2"),
                dimension: 384,
            },
            indexing: IndexingConfig {
                batch_size: 100,
                max_file_size: 1024 * 1024,
            },
        }
    }

    /// Validate configuration values
    ///
    /// # Returns
    /// Ok if configuration is valid, Err with details if invalid
    ///
    /// # Validation Rules
    /// - Database port must be between 1 and 65535
    /// - Database host cannot be empty
    /// - Embedding dimension must be positive
    /// - Batch size must be positive and <= 10000
    /// - Max file size must be positive and <= 100MB
    pub fn validate(&self) -> crate::Result<()> {
        // Validate database config
        if self.database.host.trim().is_empty() {
            return Err(crate::Error::Config(
                "Database host cannot be empty".to_string(),
            ));
        }

        if self.database.port == 0 {
            return Err(crate::Error::Config(
                "Database port cannot be 0. Must be between 1 and 65535".to_string(),
            ));
        }

        if self.database.database.trim().is_empty() {
            return Err(crate::Error::Config(
                "Database name cannot be empty".to_string(),
            ));
        }

        if self.database.user.trim().is_empty() {
            return Err(crate::Error::Config(
                "Database user cannot be empty".to_string(),
            ));
        }

        // Validate embedding config
        if self.embedding.dimension == 0 {
            return Err(crate::Error::Config(
                "Embedding dimension must be positive".to_string(),
            ));
        }

        if self.embedding.dimension > 4096 {
            return Err(crate::Error::Config(format!(
                "Embedding dimension {} is too large. Maximum is 4096",
                self.embedding.dimension
            )));
        }

        // Validate indexing config
        if self.indexing.batch_size == 0 {
            return Err(crate::Error::Config(
                "Batch size must be positive".to_string(),
            ));
        }

        if self.indexing.batch_size > 10000 {
            return Err(crate::Error::Config(format!(
                "Batch size {} is too large. Maximum is 10000",
                self.indexing.batch_size
            )));
        }

        if self.indexing.max_file_size == 0 {
            return Err(crate::Error::Config(
                "Max file size must be positive".to_string(),
            ));
        }

        const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB
        if self.indexing.max_file_size > MAX_FILE_SIZE {
            return Err(crate::Error::Config(format!(
                "Max file size {} bytes is too large. Maximum is {} bytes (100MB)",
                self.indexing.max_file_size, MAX_FILE_SIZE
            )));
        }

        Ok(())
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

/// XDG Base Directory helper functions
/// These functions check environment variables first, then fall back to XDG defaults
/// Get XDG_DATA_HOME directory (default: ~/.local/share)
fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/share"))
                .expect("HOME environment variable must be set")
        })
}

/// Get XDG_CONFIG_HOME directory (default: ~/.config)
#[allow(dead_code)]
fn xdg_config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
                .expect("HOME environment variable must be set")
        })
}

/// Get XDG_STATE_HOME directory (default: ~/.local/state)
#[allow(dead_code)]
pub fn xdg_state_home() -> PathBuf {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/state"))
                .expect("HOME environment variable must be set")
        })
}

/// Get XDG_CACHE_HOME directory (default: ~/.cache)
#[allow(dead_code)]
fn xdg_cache_home() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".cache"))
                .expect("HOME environment variable must be set")
        })
}

/// Get cudgel data directory (XDG_DATA_HOME/cudgel)
pub fn cudgel_data_dir() -> PathBuf {
    xdg_data_home().join("cudgel")
}

/// Get cudgel config directory (XDG_CONFIG_HOME/cudgel)
#[allow(dead_code)]
pub fn cudgel_config_dir() -> PathBuf {
    xdg_config_home().join("cudgel")
}

/// Get cudgel state directory (XDG_STATE_HOME/cudgel)
/// Used for orchestrator logs, PID files, etc.
#[allow(dead_code)]
pub fn cudgel_state_dir() -> PathBuf {
    xdg_state_home().join("cudgel")
}

/// Get cudgel cache directory (XDG_CACHE_HOME/cudgel)
#[allow(dead_code)]
pub fn cudgel_cache_dir() -> PathBuf {
    xdg_cache_home().join("cudgel")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_config() -> Config {
        Config {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 54321,
                database: "cudgel".to_string(),
                user: "testuser".to_string(),
                password: "password".to_string(),
            },
            embedding: EmbeddingConfig {
                model_path: PathBuf::from("/tmp/models"),
                dimension: 384,
            },
            indexing: IndexingConfig {
                batch_size: 100,
                max_file_size: 1024 * 1024,
            },
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_valid_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_database_port_zero() {
        let mut config = create_valid_config();
        config.database.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_database_empty_host() {
        let mut config = create_valid_config();
        config.database.host = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_database_whitespace_host() {
        let mut config = create_valid_config();
        config.database.host = "   ".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_database_empty_name() {
        let mut config = create_valid_config();
        config.database.database = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_database_empty_user() {
        let mut config = create_valid_config();
        config.database.user = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_embedding_dimension_zero() {
        let mut config = create_valid_config();
        config.embedding.dimension = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_embedding_dimension_too_large() {
        let mut config = create_valid_config();
        config.embedding.dimension = 5000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_embedding_dimension_max_valid() {
        let mut config = create_valid_config();
        config.embedding.dimension = 4096;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_batch_size_zero() {
        let mut config = create_valid_config();
        config.indexing.batch_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_batch_size_too_large() {
        let mut config = create_valid_config();
        config.indexing.batch_size = 20000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_batch_size_max_valid() {
        let mut config = create_valid_config();
        config.indexing.batch_size = 10000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_max_file_size_zero() {
        let mut config = create_valid_config();
        config.indexing.max_file_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_max_file_size_too_large() {
        let mut config = create_valid_config();
        config.indexing.max_file_size = 200 * 1024 * 1024; // 200MB
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_max_file_size_max_valid() {
        let mut config = create_valid_config();
        config.indexing.max_file_size = 100 * 1024 * 1024; // 100MB
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_local_creates_valid_config() {
        let config = Config::local();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_default_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_xdg_data_home_returns_absolute_path() {
        let data_home = xdg_data_home();
        assert!(data_home.is_absolute());
        assert!(!data_home.as_os_str().is_empty());
    }

    #[test]
    fn test_xdg_config_home_returns_absolute_path() {
        let config_home = xdg_config_home();
        assert!(config_home.is_absolute());
        assert!(!config_home.as_os_str().is_empty());
    }

    #[test]
    fn test_xdg_state_home_returns_absolute_path() {
        let state_home = xdg_state_home();
        assert!(state_home.is_absolute());
        assert!(!state_home.as_os_str().is_empty());
    }

    #[test]
    fn test_xdg_cache_home_returns_absolute_path() {
        let cache_home = xdg_cache_home();
        assert!(cache_home.is_absolute());
        assert!(!cache_home.as_os_str().is_empty());
    }

    #[test]
    fn test_cudgel_directories_contain_cudgel() {
        let data_dir = cudgel_data_dir();
        let config_dir = cudgel_config_dir();
        let state_dir = cudgel_state_dir();
        let cache_dir = cudgel_cache_dir();

        assert!(data_dir.to_string_lossy().contains("cudgel"));
        assert!(config_dir.to_string_lossy().contains("cudgel"));
        assert!(state_dir.to_string_lossy().contains("cudgel"));
        assert!(cache_dir.to_string_lossy().contains("cudgel"));
    }

    #[test]
    fn test_database_url_format() {
        let config = create_valid_config();
        let url = config.database_url();

        assert!(url.contains("host=localhost"));
        assert!(url.contains("port=54321"));
        assert!(url.contains("dbname=cudgel"));
        assert!(url.contains("user=testuser"));
        assert!(url.contains("password=password"));
    }
}
