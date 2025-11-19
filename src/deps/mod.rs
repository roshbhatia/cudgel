// src/deps/mod.rs
//! Dependency management module for automatic setup and validation
//!
//! This module provides automatic dependency management for cudgel, including:
//! - Model downloads from HuggingFace Hub
//! - PostgreSQL database lifecycle management
//! - Schema initialization and validation
//! - XDG-compliant file system layout

pub mod checker;
pub mod database;
pub mod model;
pub mod schema;

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Dependency status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyStatus {
    Missing,
    Satisfied,
    Corrupted,
    Unknown,
}

/// Component type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    Model,
    Database,
    Schema,
    ExternalTool,
}

/// Represents a required dependency for cudgel
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub component_type: ComponentType,
    pub status: DependencyStatus,
    pub required: bool,
    pub error_message: Option<String>,
}

impl Dependency {
    /// Create a new dependency
    pub fn new(name: impl Into<String>, component_type: ComponentType, required: bool) -> Self {
        Self {
            name: name.into(),
            component_type,
            status: DependencyStatus::Unknown,
            required,
            error_message: None,
        }
    }

    /// Check if this dependency is satisfied
    pub fn is_satisfied(&self) -> bool {
        self.status == DependencyStatus::Satisfied
    }

    /// Check if this dependency is missing
    pub fn is_missing(&self) -> bool {
        self.status == DependencyStatus::Missing
    }
}

/// Get XDG-compliant directory paths
fn get_xdg_paths() -> (PathBuf, PathBuf) {
    let xdg_data = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share", home)
    });

    let models_dir = Path::new(&xdg_data).join("cudgel/models");
    let scripts_dir = PathBuf::from("scripts");

    (models_dir, scripts_dir)
}

/// Install all dependencies with proper ordering
pub async fn install_all() -> Result<()> {
    println!("🔧 Installing cudgel dependencies...\n");

    let (models_dir, scripts_dir) = get_xdg_paths();
    let database_port = 45678;

    // Step 1: Check prerequisites first
    println!("⚙️  Checking prerequisites...");
    let checker = checker::DependencyChecker::new(models_dir.clone(), database_port);
    let prereqs = checker.check_prerequisites()?;

    let mut all_satisfied = true;
    for prereq in &prereqs {
        if !prereq.is_satisfied() {
            all_satisfied = false;
            if let Some(ref msg) = prereq.error_message {
                eprintln!("✗ {}: {}", prereq.name, msg);
            }
        } else {
            println!("✓ {}", prereq.name);
        }
    }

    if !all_satisfied {
        return Err(Error::DependencyMissing(
            "Prerequisites not satisfied. Install required tools first.".to_string(),
        ));
    }

    println!();

    // Step 2: Validate current state
    let deps = checker.validate_all().await?;
    let needs_model = deps
        .iter()
        .any(|d| d.name == "ONNX Embedding Model" && !d.is_satisfied());
    let needs_database = deps
        .iter()
        .any(|d| d.name == "PostgreSQL Database" && !d.is_satisfied());
    let needs_schema = deps
        .iter()
        .any(|d| d.name == "Database Schema" && !d.is_satisfied());

    // Step 3: Download model if needed
    if needs_model {
        println!("📦 Downloading ONNX embedding model...");
        download_model(&models_dir).await?;
        println!("✓ Model downloaded successfully\n");
    } else {
        println!("✓ Model already present\n");
    }

    // Step 4: Start database if needed
    if needs_database {
        println!("🗄️  Starting PostgreSQL database...");
        start_database(&scripts_dir, database_port).await?;
        println!("✓ Database started successfully\n");
    } else {
        println!("✓ Database already running\n");
    }

    // Step 5: Initialize schema if needed (depends on database)
    if needs_schema {
        // Wait a moment for database to be fully ready
        if needs_database {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        println!("🔨 Initializing database schema...");
        initialize_schema(database_port).await?;
        println!("✓ Schema initialized successfully\n");
    } else {
        println!("✓ Schema already initialized\n");
    }

    // Step 6: Final validation
    let final_deps = checker.validate_all().await?;
    let all_satisfied = final_deps.iter().all(|d| d.is_satisfied());

    if all_satisfied {
        println!("✅ All dependencies satisfied! Cudgel is ready to use.\n");
        Ok(())
    } else {
        println!("\n{}", checker.format_validation_table(&final_deps));
        Err(Error::DependencyMissing(
            "Some dependencies could not be satisfied".to_string(),
        ))
    }
}

/// Download model to XDG directory
async fn download_model(models_dir: &Path) -> Result<()> {
    use model::{ModelArtifact, ModelDownloader};

    // Create models directory
    std::fs::create_dir_all(models_dir)?;

    let model_dir = models_dir.join("sentence-transformers/all-MiniLM-L6-v2");
    std::fs::create_dir_all(&model_dir)?;

    let downloader = ModelDownloader::new(models_dir.to_path_buf());

    // Download model files
    let files = vec![
        ("model.onnx", 90_000_000),
        ("tokenizer.json", 5_000_000),
        ("config.json", 1_000),
    ];

    for (filename, size) in files {
        let target_path = model_dir.join(filename);
        if !target_path.exists() {
            let mut artifact = ModelArtifact::new(
                "sentence-transformers/all-MiniLM-L6-v2",
                filename,
                target_path,
                size,
            );
            downloader.download_model_artifact(&mut artifact).await?;
        }
    }

    // Verify integrity
    downloader.verify_model_integrity(&model_dir)?;

    Ok(())
}

/// Start PostgreSQL database
async fn start_database(scripts_dir: &Path, port: u16) -> Result<()> {
    let pg_manager = database::PostgresManager::new(scripts_dir.to_path_buf(), port);

    // Check if already running (idempotency)
    if pg_manager.is_running()? {
        return Ok(());
    }

    // Start database
    pg_manager.start()?;

    // Wait for startup with timeout
    pg_manager.wait_for_startup(30).await?;

    Ok(())
}

/// Initialize database schema
async fn initialize_schema(port: u16) -> Result<()> {
    let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
    let initializer =
        schema::SchemaInitializer::new("localhost".to_string(), port, "cudgel".to_string(), user);

    // Check if already initialized (idempotency)
    if initializer.check_initialized().await? {
        return Ok(());
    }

    // Initialize schema
    initializer.initialize_schema().await?;

    // Verify extensions
    initializer.verify_extensions().await?;

    Ok(())
}

/// Validate all dependencies without modifying system
pub async fn validate_only() -> Result<Vec<Dependency>> {
    let (models_dir, _) = get_xdg_paths();
    let database_port = 45678;

    let checker = checker::DependencyChecker::new(models_dir, database_port);
    checker.validate_all().await
}

/// Clean downloaded models and temporary files
pub async fn clean_models() -> Result<()> {
    let (models_dir, _) = get_xdg_paths();

    if models_dir.exists() {
        std::fs::remove_dir_all(&models_dir)?;
        println!("✓ Removed models directory: {}", models_dir.display());
    } else {
        println!("ℹ  Models directory does not exist");
    }

    Ok(())
}

/// Clean all data including database
pub async fn clean_all() -> Result<()> {
    let (models_dir, scripts_dir) = get_xdg_paths();
    let database_port = 45678;

    // Stop database first
    let pg_manager = database::PostgresManager::new(scripts_dir, database_port);
    if pg_manager.is_running()? {
        println!("Stopping PostgreSQL...");
        pg_manager.stop()?;
    }

    // Remove models
    if models_dir.exists() {
        std::fs::remove_dir_all(&models_dir)?;
        println!("✓ Removed models directory");
    }

    // Remove PostgreSQL data directory
    let xdg_data = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.local/share", home)
    });
    let pg_data_dir = Path::new(&xdg_data).join("cudgel/postgres");
    if pg_data_dir.exists() {
        std::fs::remove_dir_all(&pg_data_dir)?;
        println!("✓ Removed PostgreSQL data directory");
    }

    println!("✅ All cudgel data cleaned");

    Ok(())
}
