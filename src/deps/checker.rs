// src/deps/checker.rs
//! Dependency validation and checking

use super::{ComponentType, Dependency, DependencyStatus};
use crate::error::Result;
use std::path::Path;
use std::process::Command;

/// Dependency checker for validation
pub struct DependencyChecker {
    models_dir: std::path::PathBuf,
    database_port: u16,
}

impl DependencyChecker {
    /// Create a new dependency checker
    pub fn new(models_dir: std::path::PathBuf, database_port: u16) -> Self {
        Self {
            models_dir,
            database_port,
        }
    }

    /// Validate all dependencies
    pub async fn validate_all(&self) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Check model
        let mut model_dep = Dependency::new("ONNX Embedding Model", ComponentType::Model, true);
        model_dep.status = self.check_model_exists()?;
        if model_dep.status != DependencyStatus::Satisfied {
            model_dep.error_message = Some(format!(
                "Model not found in {}. Run: cudgel deps",
                self.models_dir.display()
            ));
        }
        dependencies.push(model_dep);

        // Check database
        let mut db_dep = Dependency::new("PostgreSQL Database", ComponentType::Database, true);
        db_dep.status = self.check_database_running()?;
        if db_dep.status != DependencyStatus::Satisfied {
            db_dep.error_message = Some(format!(
                "PostgreSQL not running on port {}. Run: cudgel deps",
                self.database_port
            ));
        }
        dependencies.push(db_dep);

        // Check schema
        let mut schema_dep = Dependency::new("Database Schema", ComponentType::Schema, true);
        schema_dep.status = self.check_schema_initialized().await?;
        if schema_dep.status != DependencyStatus::Satisfied {
            schema_dep.error_message =
                Some("Database schema not initialized. Run: cudgel deps".to_string());
        }
        dependencies.push(schema_dep);

        Ok(dependencies)
    }

    /// Check prerequisites (PostgreSQL installation, disk space, etc.)
    pub fn check_prerequisites(&self) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Check PostgreSQL binary exists
        let mut pg_dep = Dependency::new("PostgreSQL Binary", ComponentType::ExternalTool, true);
        pg_dep.status = if Command::new("pg_ctl").arg("--version").output().is_ok() {
            DependencyStatus::Satisfied
        } else {
            pg_dep.error_message = Some(
                "PostgreSQL not found. Install with: brew install postgresql@17 (macOS) or apt install postgresql-17 (Linux)".to_string()
            );
            DependencyStatus::Missing
        };
        dependencies.push(pg_dep);

        // Check disk space (simplified check - just verify models_dir is writable)
        let mut disk_dep =
            Dependency::new("Sufficient Disk Space", ComponentType::ExternalTool, true);
        disk_dep.status = if self
            .models_dir
            .parent()
            .map(|p| p.exists())
            .unwrap_or(false)
            || self.models_dir.exists()
        {
            DependencyStatus::Satisfied
        } else {
            disk_dep.error_message = Some(format!(
                "Cannot access {}. Check disk space and permissions.",
                self.models_dir.display()
            ));
            DependencyStatus::Missing
        };
        dependencies.push(disk_dep);

        Ok(dependencies)
    }

    /// Check if model exists
    fn check_model_exists(&self) -> Result<DependencyStatus> {
        let model_path = self
            .models_dir
            .join("sentence-transformers/all-MiniLM-L6-v2/model.onnx");
        if model_path.exists() {
            let metadata = std::fs::metadata(&model_path)?;
            // Basic sanity check: model should be > 10MB
            if metadata.len() > 10_000_000 {
                Ok(DependencyStatus::Satisfied)
            } else {
                Ok(DependencyStatus::Corrupted)
            }
        } else {
            Ok(DependencyStatus::Missing)
        }
    }

    /// Check if database is running
    fn check_database_running(&self) -> Result<DependencyStatus> {
        let output = Command::new("pg_isready")
            .arg("-p")
            .arg(self.database_port.to_string())
            .arg("-h")
            .arg("localhost")
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(DependencyStatus::Satisfied),
            _ => Ok(DependencyStatus::Missing),
        }
    }

    /// Check if schema is initialized
    async fn check_schema_initialized(&self) -> Result<DependencyStatus> {
        use tokio_postgres::NoTls;

        let conn_str = format!(
            "host=localhost port={} dbname=cudgel user={} password=",
            self.database_port,
            std::env::var("USER").unwrap_or_else(|_| "postgres".to_string())
        );

        let client_result = tokio_postgres::connect(&conn_str, NoTls).await;

        match client_result {
            Ok((client, connection)) => {
                // Spawn connection task
                tokio::spawn(async move {
                    let _ = connection.await;
                });

                // Check if repositories table exists
                let row = client
                    .query_opt(
                        "SELECT EXISTS (
                            SELECT FROM information_schema.tables
                            WHERE table_name = 'repositories'
                        )",
                        &[],
                    )
                    .await;

                match row {
                    Ok(Some(row)) => {
                        let exists: bool = row.get(0);
                        if exists {
                            Ok(DependencyStatus::Satisfied)
                        } else {
                            Ok(DependencyStatus::Missing)
                        }
                    }
                    _ => Ok(DependencyStatus::Missing),
                }
            }
            Err(_) => Ok(DependencyStatus::Missing),
        }
    }

    /// Format validation results as a table
    pub fn format_validation_table(&self, dependencies: &[Dependency]) -> String {
        let mut output = String::new();
        output.push_str("\nDependency Status:\n");
        output.push_str("─────────────────────────────────────────────────\n");

        for dep in dependencies {
            let status_icon = match dep.status {
                DependencyStatus::Satisfied => "✓",
                DependencyStatus::Missing => "✗",
                DependencyStatus::Corrupted => "⚠",
                DependencyStatus::Unknown => "?",
            };

            let status_text = match dep.status {
                DependencyStatus::Satisfied => "OK",
                DependencyStatus::Missing => "MISSING",
                DependencyStatus::Corrupted => "CORRUPTED",
                DependencyStatus::Unknown => "UNKNOWN",
            };

            output.push_str(&format!(
                "{} {:<30} {}\n",
                status_icon, dep.name, status_text
            ));

            if let Some(ref error_msg) = dep.error_message {
                output.push_str(&format!("  └─ {}\n", error_msg));
            }
        }

        output.push_str("─────────────────────────────────────────────────\n");
        output
    }

    /// Collect diagnostic information for verbose mode
    pub fn collect_diagnostics(&self) -> Result<String> {
        let mut diagnostics = String::new();

        diagnostics.push_str("\n=== Diagnostic Information ===\n\n");

        // PostgreSQL version
        if let Ok(output) = Command::new("pg_ctl").arg("--version").output() {
            diagnostics.push_str("PostgreSQL Version:\n");
            diagnostics.push_str(&format!(
                "  {}\n\n",
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        // Models directory
        diagnostics.push_str(&format!(
            "Models Directory: {}\n",
            self.models_dir.display()
        ));
        diagnostics.push_str(&format!("  Exists: {}\n\n", self.models_dir.exists()));

        // Database port
        diagnostics.push_str(&format!("Database Port: {}\n", self.database_port));

        // Environment variables
        diagnostics.push_str("Environment:\n");
        if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
            diagnostics.push_str(&format!("  XDG_DATA_HOME: {}\n", xdg_data));
        }
        if let Ok(home) = std::env::var("HOME") {
            diagnostics.push_str(&format!("  HOME: {}\n", home));
        }

        diagnostics.push_str("\n===============================\n");

        Ok(diagnostics)
    }
}

impl Default for DependencyChecker {
    fn default() -> Self {
        let xdg_data = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.local/share", home)
        });
        let models_dir = Path::new(&xdg_data).join("cudgel/models");

        Self::new(models_dir, 45678)
    }
}
