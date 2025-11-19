// src/kg/client.rs
//! Knowledge graph database client implementation.

use super::{
    CodeEntity, Component, DependencyType, EntityMatch, EntityRelationships, Repository,
    RepositoryStats, Result,
};
use async_trait::async_trait;

/// Record ID type alias for SurrealDB record identifiers
pub type RecordId = String;

/// Trait defining knowledge graph database operations
#[async_trait]
pub trait KgClient: Send + Sync {
    // === Repository Operations ===

    /// Create a new repository node
    async fn create_repository(&self, repo: Repository) -> Result<RecordId>;

    /// Get repository by path
    async fn get_repository_by_path(&self, path: &str) -> Result<Option<Repository>>;

    /// Update repository summary
    async fn update_repository_summary(&self, repo_id: &RecordId, summary: String)
        -> Result<()>;

    // === Component Operations ===

    /// Create a new component node
    async fn create_component(&self, component: Component) -> Result<RecordId>;

    /// Get all components in a repository
    async fn get_components(&self, repo_id: &RecordId) -> Result<Vec<Component>>;

    /// Update component summary
    async fn update_component_summary(&self, component_id: &RecordId, summary: String)
        -> Result<()>;

    // === Entity Operations ===

    /// Create a new code entity node
    async fn create_entity(&self, entity: CodeEntity) -> Result<RecordId>;

    /// Batch create multiple entities (for performance)
    async fn create_entities_batch(&self, entities: Vec<CodeEntity>) -> Result<Vec<RecordId>>;

    /// Get entity by ID
    async fn get_entity(&self, entity_id: &RecordId) -> Result<Option<CodeEntity>>;

    /// Find entities by name (exact match)
    async fn find_entities_by_name(
        &self,
        repo_id: &RecordId,
        name: &str,
    ) -> Result<Vec<CodeEntity>>;

    /// Find entities by name pattern (fuzzy match)
    async fn search_entities_by_name(
        &self,
        repo_id: &RecordId,
        pattern: &str,
        threshold: f64,
    ) -> Result<Vec<EntityMatch>>;

    /// Get entities in a file
    async fn get_entities_by_file(
        &self,
        repo_id: &RecordId,
        file_path: &str,
    ) -> Result<Vec<CodeEntity>>;

    /// Get all entity names (for fuzzy matching)
    async fn get_all_entity_names(&self, repo_id: &RecordId) -> Result<Vec<String>>;

    /// Update entity summary
    async fn update_entity_summary(&self, entity_id: &RecordId, summary: String) -> Result<()>;

    /// Delete entity and cascade delete relationships
    async fn delete_entity_cascade(&self, entity_id: &RecordId) -> Result<()>;

    // === Relationship Operations ===

    /// Create a DEPENDS_ON relationship
    async fn create_dependency(
        &self,
        from: &RecordId,
        to: &RecordId,
        dep_type: DependencyType,
    ) -> Result<RecordId>;

    /// Create a USES relationship
    async fn create_uses(&self, from: &RecordId, to: &RecordId, context: String)
        -> Result<RecordId>;

    /// Create a CONTAINS relationship
    async fn create_contains(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;

    /// Create an IMPLEMENTS relationship
    async fn create_implements(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;

    /// Create a CALLS relationship
    async fn create_calls(&self, from: &RecordId, to: &RecordId, call_count: usize)
        -> Result<RecordId>;

    /// Get outgoing relationships for an entity
    async fn get_outgoing_relationships(
        &self,
        entity_id: &RecordId,
    ) -> Result<EntityRelationships>;

    /// Get incoming relationships for an entity
    async fn get_incoming_relationships(
        &self,
        entity_id: &RecordId,
    ) -> Result<EntityRelationships>;

    /// Get all relationships for an entity (both directions)
    async fn get_all_relationships(&self, entity_id: &RecordId) -> Result<EntityRelationships>;

    /// Traverse dependencies (multi-hop, up to max_depth)
    async fn traverse_dependencies(
        &self,
        entity_id: &RecordId,
        max_depth: usize,
    ) -> Result<Vec<CodeEntity>>;

    // === Query Operations ===

    /// Execute arbitrary SurrealQL query (for complex queries)
    async fn execute_query(&self, query: &str) -> Result<Vec<serde_json::Value>>;

    /// Get repository statistics
    async fn get_repository_stats(&self, repo_id: &RecordId) -> Result<RepositoryStats>;

    // === Maintenance Operations ===

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<()>;

    /// Check if schema is initialized
    async fn is_schema_initialized(&self) -> Result<bool>;

    /// Vacuum/optimize database (cleanup unused space)
    async fn optimize(&self) -> Result<()>;
}
