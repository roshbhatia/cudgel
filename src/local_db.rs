//! Local PostgreSQL database management
//!
//! This module provides functionality to manage a local PostgreSQL instance
//! using Docker Compose. It handles database lifecycle including:
//! - Starting/stopping the database
//! - Health checking
//! - Automatic initialization
//! - Data persistence
//!
//! # Example
//!
//! ```no_run
//! use cudgel::local_db::LocalDatabase;
//!
//! # async fn example() -> cudgel::Result<()> {
//! let local_db = LocalDatabase::new();
//!
//! // Start the database (idempotent)
//! local_db.start().await?;
//!
//! // Check if it's running
//! if local_db.is_running().await? {
//!     println!("Database is ready!");
//! }
//! # Ok(())
//! # }
//! ```

use crate::{Config, Result};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Default Docker Compose project name
const PROJECT_NAME: &str = "cudgel";

/// Maximum time to wait for database to become healthy (seconds)
const MAX_WAIT_TIME: u64 = 60;

/// Manages a local PostgreSQL database instance using Docker Compose
pub struct LocalDatabase {
    compose_file: String,
}

impl LocalDatabase {
    /// Create a new LocalDatabase manager
    ///
    /// Uses the docker-compose.yml file in the current directory.
    pub fn new() -> Self {
        LocalDatabase {
            compose_file: "docker-compose.yml".to_string(),
        }
    }

    /// Create a LocalDatabase manager with a custom docker-compose file
    pub fn with_compose_file(compose_file: String) -> Self {
        LocalDatabase { compose_file }
    }

    /// Check if Docker is available on the system
    ///
    /// # Errors
    ///
    /// Returns an error if Docker is not installed or not running
    pub fn check_docker(&self) -> Result<bool> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .map_err(|e| crate::Error::Other(format!("Docker not found: {}", e)))?;

        Ok(output.status.success())
    }

    /// Check if the database container is running
    ///
    /// # Errors
    ///
    /// Returns an error if Docker command fails
    pub async fn is_running(&self) -> Result<bool> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=cudgel-postgres",
                "--format",
                "{{.Names}}",
            ])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to check container status: {}", e)))?;

        let container_name = String::from_utf8_lossy(&output.stdout);
        Ok(container_name.trim() == "cudgel-postgres")
    }

    /// Check if the database is healthy and ready to accept connections
    ///
    /// # Errors
    ///
    /// Returns an error if the database is not healthy
    pub async fn is_healthy(&self) -> Result<bool> {
        if !self.is_running().await? {
            return Ok(false);
        }

        let output = Command::new("docker")
            .args([
                "exec",
                "cudgel-postgres",
                "pg_isready",
                "-U",
                "cudgel",
                "-d",
                "cudgel",
            ])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to check database health: {}", e)))?;

        Ok(output.status.success())
    }

    /// Start the local database using Docker Compose
    ///
    /// This operation is idempotent - it's safe to call even if the database
    /// is already running.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Docker is not available
    /// - Docker Compose file is missing
    /// - Database fails to start
    /// - Database doesn't become healthy within timeout
    pub async fn start(&self) -> Result<()> {
        // Check Docker availability
        if !self.check_docker()? {
            return Err(crate::Error::Other(
                "Docker is not available. Please install Docker to use local database.".to_string(),
            ));
        }

        // Check if already running
        if self.is_running().await? {
            println!("Database is already running");
            return self.wait_for_healthy().await;
        }

        println!("Starting local PostgreSQL database with pgvector...");

        // Start the postgres service only (not Temporal)
        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file,
                "-p",
                PROJECT_NAME,
                "up",
                "-d",
                "postgres",
            ])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to start database: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to start database: {}",
                stderr
            )));
        }

        println!("Waiting for database to become healthy...");
        self.wait_for_healthy().await?;

        println!(" Local database is ready!");
        Ok(())
    }

    /// Wait for the database to become healthy
    ///
    /// Polls the database health every 2 seconds up to MAX_WAIT_TIME seconds.
    async fn wait_for_healthy(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(MAX_WAIT_TIME);

        while start.elapsed() < timeout {
            if self.is_healthy().await? {
                return Ok(());
            }
            sleep(Duration::from_secs(2)).await;
        }

        Err(crate::Error::Other(format!(
            "Database did not become healthy within {} seconds",
            MAX_WAIT_TIME
        )))
    }

    /// Stop the local database
    ///
    /// # Errors
    ///
    /// Returns an error if Docker Compose command fails
    pub async fn stop(&self) -> Result<()> {
        println!("Stopping local database...");

        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file,
                "-p",
                PROJECT_NAME,
                "stop",
                "postgres",
            ])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to stop database: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to stop database: {}",
                stderr
            )));
        }

        println!(" Database stopped");
        Ok(())
    }

    /// Remove the local database and all data
    ///
    /// **Warning**: This will delete all indexed data!
    ///
    /// # Errors
    ///
    /// Returns an error if Docker Compose command fails
    pub async fn remove(&self) -> Result<()> {
        println!("Removing local database and all data...");

        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file,
                "-p",
                PROJECT_NAME,
                "down",
                "-v",
            ])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to remove database: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to remove database: {}",
                stderr
            )));
        }

        println!(" Database and data removed");
        Ok(())
    }

    /// Get the status of the local database
    ///
    /// Returns a human-readable status string
    pub async fn status(&self) -> Result<String> {
        if !self.is_running().await? {
            return Ok("stopped".to_string());
        }

        if self.is_healthy().await? {
            Ok("running (healthy)".to_string())
        } else {
            Ok("running (unhealthy)".to_string())
        }
    }

    /// Get connection configuration for the local database
    pub fn get_config() -> Config {
        let mut config = Config::default();
        config.database.host = "localhost".to_string();
        config.database.port = 5432;
        config.database.database = "cudgel".to_string();
        config.database.user = "cudgel".to_string();
        config.database.password = "cudgel".to_string();
        config
    }

    /// Ensure the local database is running and healthy
    ///
    /// This is a convenience method that starts the database if it's not running
    /// and waits for it to become healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be started or doesn't become healthy
    pub async fn ensure_running(&self) -> Result<()> {
        if !self.is_running().await? {
            self.start().await?;
        } else if !self.is_healthy().await? {
            println!("Database is running but unhealthy, waiting for it to recover...");
            self.wait_for_healthy().await?;
        }
        Ok(())
    }
}

impl Default for LocalDatabase {
    fn default() -> Self {
        Self::new()
    }
}
