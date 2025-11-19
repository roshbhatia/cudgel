// src/deps/database.rs
//! PostgreSQL database lifecycle management

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Database instance status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Represents a PostgreSQL database instance
#[derive(Debug, Clone)]
pub struct DatabaseInstance {
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub data_directory: PathBuf,
    pub pid: Option<u32>,
    pub status: DatabaseStatus,
    pub log_file: PathBuf,
    pub version: Option<String>,
}

impl DatabaseInstance {
    /// Create a new database instance configuration
    pub fn new(host: String, port: u16, data_directory: PathBuf, log_file: PathBuf) -> Self {
        Self {
            host,
            port,
            database_name: "cudgel".to_string(),
            data_directory,
            pid: None,
            status: DatabaseStatus::Stopped,
            log_file,
            version: None,
        }
    }
}

/// PostgreSQL manager for lifecycle operations
pub struct PostgresManager {
    scripts_dir: PathBuf,
    port: u16,
}

impl PostgresManager {
    /// Create a new PostgreSQL manager
    pub fn new(scripts_dir: PathBuf, port: u16) -> Self {
        Self { scripts_dir, port }
    }

    /// Check if PostgreSQL is running using pg_isready
    pub fn is_running(&self) -> Result<bool> {
        let output = Command::new("pg_isready")
            .arg("-p")
            .arg(self.port.to_string())
            .arg("-h")
            .arg("localhost")
            .output();

        match output {
            Ok(out) => Ok(out.status.success()),
            Err(_) => {
                // pg_isready not found or other error - assume not running
                Ok(false)
            }
        }
    }

    /// Start PostgreSQL using the start-postgres.sh script
    pub fn start(&self) -> Result<()> {
        let script_path = self.scripts_dir.join("start-postgres.sh");

        if !script_path.exists() {
            return Err(Error::DatabaseStartFailed(format!(
                "Start script not found: {}",
                script_path.display()
            )));
        }

        let output = Command::new(&script_path)
            .env("CUDGEL_POSTGRES_PORT", self.port.to_string())
            .output()
            .map_err(|e| {
                Error::DatabaseStartFailed(format!("Failed to execute start script: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::DatabaseStartFailed(format!(
                "Start script failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Stop PostgreSQL using the stop-postgres.sh script
    pub fn stop(&self) -> Result<()> {
        let script_path = self.scripts_dir.join("stop-postgres.sh");

        if !script_path.exists() {
            return Err(Error::DatabaseStopFailed(format!(
                "Stop script not found: {}",
                script_path.display()
            )));
        }

        let output = Command::new(&script_path)
            .env("CUDGEL_POSTGRES_PORT", self.port.to_string())
            .output()
            .map_err(|e| {
                Error::DatabaseStopFailed(format!("Failed to execute stop script: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Stopping when already stopped is not an error
            if !stderr.contains("is not running") {
                return Err(Error::DatabaseStopFailed(format!(
                    "Stop script failed: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    /// Detect if port is in use by another process
    pub fn detect_port_conflict(&self) -> Result<Option<u32>> {
        // Use lsof to check if the port is in use (Unix-only)
        #[cfg(unix)]
        {
            let output = Command::new("lsof")
                .arg("-i")
                .arg(format!(":{}", self.port))
                .arg("-t")
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if let Ok(pid) = stdout.trim().parse::<u32>() {
                        return Ok(Some(pid));
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily detect port conflicts
            // Just check if PostgreSQL is running
            if self.is_running()? {
                return Ok(Some(0)); // Return dummy PID
            }
        }

        Ok(None)
    }

    /// Wait for database to start (with timeout)
    pub async fn wait_for_startup(&self, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if self.is_running()? {
                return Ok(());
            }
            sleep(Duration::from_secs(1)).await;
        }

        Err(Error::DatabaseStartFailed(format!(
            "Database failed to start within {} seconds",
            timeout_secs
        )))
    }
}
