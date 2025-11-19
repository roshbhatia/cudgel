// src/kg/mod.rs
//! Knowledge graph module for code understanding.
//!
//! This module provides functionality to build and query a knowledge graph
//! of code entities, their relationships, and summaries.

pub mod client;
pub mod model;

pub use client::KgClient;
pub use model::{
    CodeEntity, Component, ComponentType, DependencyType, EntityMatch, EntityMetadata,
    EntityRelationships, EntityType, RelatedEntity, Repository, RepositoryStats, Visibility,
};

use thiserror::Error;

/// Errors that can occur during knowledge graph operations
#[derive(Debug, Error)]
pub enum KgError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl KgError {
    /// Convert error to user-friendly message with troubleshooting steps
    pub fn to_user_message(&self) -> String {
        match self {
            KgError::Database(msg) => {
                format!(
                    "Database error: {}\n\nTroubleshooting:\n- Ensure the graph database is initialized\n- Check disk space and permissions",
                    msg
                )
            }
            KgError::NotFound(msg) => {
                format!(
                    "Entity not found: {}\n\nTroubleshooting:\n- Verify the repository is indexed with --enable-graph\n- Check entity name spelling",
                    msg
                )
            }
            KgError::InvalidInput(msg) => {
                format!(
                    "Invalid input: {}\n\nTroubleshooting:\n- Check input format and constraints\n- Entity names must be 1-255 characters",
                    msg
                )
            }
            KgError::Schema(msg) => {
                format!(
                    "Schema error: {}\n\nTroubleshooting:\n- Re-initialize the database\n- Check SurrealDB version compatibility",
                    msg
                )
            }
            KgError::Query(msg) => {
                format!(
                    "Query error: {}\n\nTroubleshooting:\n- Simplify your query\n- Check query syntax",
                    msg
                )
            }
            KgError::Serialization(msg) => {
                format!(
                    "Serialization error: {}\n\nTroubleshooting:\n- Check data format\n- Verify data integrity",
                    msg
                )
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, KgError>;
