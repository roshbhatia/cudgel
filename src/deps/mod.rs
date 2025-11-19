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

/// Install all dependencies
pub async fn install_all() -> Result<()> {
    // Implementation in later phase
    todo!("implement install_all")
}

/// Validate all dependencies without modifying system
pub async fn validate_only() -> Result<Vec<Dependency>> {
    // Implementation in later phase
    todo!("implement validate_only")
}

/// Clean downloaded models and temporary files
pub async fn clean_models() -> Result<()> {
    // Implementation in later phase
    todo!("implement clean_models")
}

/// Clean all data including database
pub async fn clean_all() -> Result<()> {
    // Implementation in later phase
    todo!("implement clean_all")
}
