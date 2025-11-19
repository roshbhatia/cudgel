// src/kg/model.rs
//! Data models for the knowledge graph.

use serde::{Deserialize, Serialize};

/// Represents a code repository in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub path: String,
    pub name: String,
    pub summary: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Represents a logical component (module/package) in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub repository_id: String,
    pub name: String,
    pub path: String,
    pub component_type: ComponentType,
    pub summary: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Type of component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentType {
    Module,
    Package,
    Directory,
}

/// Represents a code entity (class, function, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntity {
    pub id: String,
    pub component_id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub visibility: Visibility,
    pub metadata: EntityMetadata,
    pub summary: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Type of code entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Method,
    Constant,
    Variable,
}

/// Visibility level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Metadata associated with an entity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityMetadata {
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub language: String,
}
