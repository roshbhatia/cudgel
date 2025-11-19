// src/kg/model.rs
//! Data models for the knowledge graph.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

/// Relationship types between entities

/// Type of dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Import,
    Inheritance,
    Composition,
    Association,
}

/// Result of fuzzy entity search with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMatch {
    pub entity: CodeEntity,
    pub confidence: f64, // 0.0 to 1.0
}

/// Aggregated relationships for an entity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityRelationships {
    pub dependencies: Vec<RelatedEntity>,     // Entities this depends on
    pub dependents: Vec<RelatedEntity>,       // Entities that depend on this
    pub uses: Vec<RelatedEntity>,             // Entities this uses
    pub used_by: Vec<RelatedEntity>,          // Entities that use this
    pub calls: Vec<RelatedEntity>,            // Functions this calls
    pub called_by: Vec<RelatedEntity>,        // Functions that call this
    pub implements: Vec<RelatedEntity>,       // Interfaces/traits this implements
    pub implemented_by: Vec<RelatedEntity>,   // Entities that implement this
}

/// A related entity with relationship metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntity {
    pub entity: CodeEntity,
    pub relationship_type: String, // "depends_on", "uses", "calls", etc.
    pub metadata: serde_json::Value, // Additional relationship metadata
}

/// Statistics about the repository graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub entity_count: usize,
    pub component_count: usize,
    pub relationship_count: usize,
    pub entities_by_type: HashMap<EntityType, usize>,
    pub components_by_type: HashMap<ComponentType, usize>,
    pub relationships_by_type: HashMap<String, usize>,
    pub entities_with_summaries: usize,
    pub average_dependencies_per_entity: f64,
}
