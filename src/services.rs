//! Auto-managed local services for Cudgel
//!
//! Handles automatic startup and management of PostgreSQL and Temporal using Docker Compose.
//! No configuration needed - everything "just works" for local-only usage.

use crate::Result;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Manages local PostgreSQL and Temporal services using Docker Compose
pub struct ServiceManager {
    compose_file: String,
}

impl ServiceManager {
    pub fn new() -> Self {
        // Create docker-compose.yml content inline
        let compose_content = r#"
version: '3.8'

services:
  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_USER: cudgel
      POSTGRES_PASSWORD: cudgel
      POSTGRES_DB: cudgel
    ports:
      - "5432:5432"
    volumes:
      - cudgel_pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cudgel"]
      interval: 5s
      timeout: 5s
      retries: 5

  temporal:
    image: temporalio/auto-setup:latest
    environment:
      - DB=postgresql
      - DB_PORT=5432
      - POSTGRES_USER=cudgel
      - POSTGRES_PWD=cudgel
      - POSTGRES_SEEDS=postgres
    ports:
      - "7233:7233"
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  cudgel_pgdata:
"#;

        Self {
            compose_file: compose_content.to_string(),
        }
    }

    /// Ensure all services are running, starting them if needed
    pub async fn ensure_running(&self) -> Result<()> {
        if !self.is_running().await? {
            println!("Starting local services (PostgreSQL + Temporal)...");
            self.start().await?;
            self.wait_for_healthy().await?;
            println!("Services ready!");
        }
        Ok(())
    }

    /// Start all services
    pub async fn start(&self) -> Result<()> {
        // Write compose file to temp location
        let compose_path = std::env::temp_dir().join("cudgel-compose.yml");
        std::fs::write(&compose_path, &self.compose_file)?;

        let output = Command::new("docker")
            .args(["compose", "-f"])
            .arg(&compose_path)
            .args(["-p", "cudgel", "up", "-d"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to start services: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Stop all services
    pub async fn stop(&self) -> Result<()> {
        let compose_path = std::env::temp_dir().join("cudgel-compose.yml");

        let output = Command::new("docker")
            .args(["compose", "-f"])
            .arg(&compose_path)
            .args(["-p", "cudgel", "down"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to stop services: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Check if services are running
    pub async fn is_running(&self) -> Result<bool> {
        let output = Command::new("docker")
            .args(["ps", "--filter", "name=cudgel", "--format", "{{.Names}}"])
            .output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("cudgel"))
    }

    /// Wait for services to be healthy
    async fn wait_for_healthy(&self) -> Result<()> {
        let max_wait = Duration::from_secs(60);
        let start = std::time::Instant::now();

        while start.elapsed() < max_wait {
            if self.is_healthy().await? {
                return Ok(());
            }
            sleep(Duration::from_secs(2)).await;
        }

        Err(crate::Error::Other(
            "Timeout waiting for services to be healthy".to_string(),
        ))
    }

    /// Check if services are healthy
    async fn is_healthy(&self) -> Result<bool> {
        // Check postgres health
        let pg_output = Command::new("docker")
            .args(["exec", "cudgel-postgres-1", "pg_isready", "-U", "cudgel"])
            .output();

        if let Ok(output) = pg_output {
            if !output.status.success() {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        // Temporal takes longer to start, just check if container is running
        let temporal_output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=cudgel-temporal",
                "--format",
                "{{.Status}}",
            ])
            .output()?;

        let status = String::from_utf8_lossy(&temporal_output.stdout);
        Ok(status.contains("Up"))
    }

    /// Get service status
    pub async fn status(&self) -> Result<String> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=cudgel",
                "--format",
                "table {{.Names}}\t{{.Status}}\t{{.Ports}}",
            ])
            .output()?;

        if !output.status.success() {
            return Err(crate::Error::Other(
                "Failed to get service status".to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Remove all services and data
    pub async fn remove(&self) -> Result<()> {
        let compose_path = std::env::temp_dir().join("cudgel-compose.yml");
        std::fs::write(&compose_path, &self.compose_file)?;

        let output = Command::new("docker")
            .args(["compose", "-f"])
            .arg(&compose_path)
            .args(["-p", "cudgel", "down", "-v"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Other(format!(
                "Failed to remove services: {}",
                stderr
            )));
        }

        Ok(())
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
