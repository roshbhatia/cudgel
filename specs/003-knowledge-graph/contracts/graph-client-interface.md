# Contract: Graph Database Client Interface

**Feature**: 003-knowledge-graph  
**Date**: 2025-11-19

## Overview

This contract defines the interface for interacting with the graph database (SurrealDB). The interface abstracts graph operations and provides type-safe access to the knowledge graph.

---

## Interface Definition

### GraphClient

The primary interface for all graph database operations.

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

#[async_trait]
pub trait GraphClient: Send + Sync {
    // === Repository Operations ===
    
    /// Create a new repository node
    async fn create_repository(&self, repo: Repository) -> Result<RecordId>;
    
    /// Get repository by path
    async fn get_repository_by_path(&self, path: &str) -> Result<Option<Repository>>;
    
    /// Update repository summary
    async fn update_repository_summary(&self, repo_id: &RecordId, summary: String) -> Result<()>;
    
    // === Component Operations ===
    
    /// Create a new component node
    async fn create_component(&self, component: Component) -> Result<RecordId>;
    
    /// Get all components in a repository
    async fn get_components(&self, repo_id: &RecordId) -> Result<Vec<Component>>;
    
    /// Update component summary
    async fn update_component_summary(&self, component_id: &RecordId, summary: String) -> Result<()>;
    
    // === Entity Operations ===
    
    /// Create a new code entity node
    async fn create_entity(&self, entity: CodeEntity) -> Result<RecordId>;
    
    /// Batch create multiple entities (for performance)
    async fn create_entities_batch(&self, entities: Vec<CodeEntity>) -> Result<Vec<RecordId>>;
    
    /// Get entity by ID
    async fn get_entity(&self, entity_id: &RecordId) -> Result<Option<CodeEntity>>;
    
    /// Find entities by name (exact match)
    async fn find_entities_by_name(&self, repo_id: &RecordId, name: &str) -> Result<Vec<CodeEntity>>;
    
    /// Find entities by name pattern (fuzzy match)
    async fn search_entities_by_name(&self, repo_id: &RecordId, pattern: &str, threshold: f64) -> Result<Vec<EntityMatch>>;
    
    /// Get entities in a file
    async fn get_entities_by_file(&self, repo_id: &RecordId, file_path: &str) -> Result<Vec<CodeEntity>>;
    
    /// Get all entity names (for fuzzy matching)
    async fn get_all_entity_names(&self, repo_id: &RecordId) -> Result<Vec<String>>;
    
    /// Update entity summary
    async fn update_entity_summary(&self, entity_id: &RecordId, summary: String) -> Result<()>;
    
    /// Delete entity and cascade delete relationships
    async fn delete_entity_cascade(&self, entity_id: &RecordId) -> Result<()>;
    
    // === Relationship Operations ===
    
    /// Create a DEPENDS_ON relationship
    async fn create_dependency(&self, from: &RecordId, to: &RecordId, dep_type: DependencyType) -> Result<RecordId>;
    
    /// Create a USES relationship
    async fn create_uses(&self, from: &RecordId, to: &RecordId, context: String) -> Result<RecordId>;
    
    /// Create a CONTAINS relationship
    async fn create_contains(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;
    
    /// Create an IMPLEMENTS relationship
    async fn create_implements(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;
    
    /// Create a CALLS relationship
    async fn create_calls(&self, from: &RecordId, to: &RecordId, call_count: usize) -> Result<RecordId>;
    
    /// Get outgoing relationships for an entity
    async fn get_outgoing_relationships(&self, entity_id: &RecordId) -> Result<EntityRelationships>;
    
    /// Get incoming relationships for an entity
    async fn get_incoming_relationships(&self, entity_id: &RecordId) -> Result<EntityRelationships>;
    
    /// Get all relationships for an entity (both directions)
    async fn get_all_relationships(&self, entity_id: &RecordId) -> Result<EntityRelationships>;
    
    /// Traverse dependencies (multi-hop, up to max_depth)
    async fn traverse_dependencies(&self, entity_id: &RecordId, max_depth: usize) -> Result<Vec<CodeEntity>>;
    
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
```

---

## Data Types

### EntityMatch

Result of fuzzy entity search with confidence score.

```rust
pub struct EntityMatch {
    pub entity: CodeEntity,
    pub confidence: f64,    // 0.0 to 1.0
}
```

---

### EntityRelationships

Aggregated relationships for an entity.

```rust
pub struct EntityRelationships {
    pub dependencies: Vec<RelatedEntity>,   // Entities this depends on
    pub dependents: Vec<RelatedEntity>,     // Entities that depend on this
    pub uses: Vec<RelatedEntity>,           // Entities this uses
    pub used_by: Vec<RelatedEntity>,        // Entities that use this
    pub calls: Vec<RelatedEntity>,          // Functions this calls
    pub called_by: Vec<RelatedEntity>,      // Functions that call this
    pub implements: Vec<RelatedEntity>,     // Interfaces/traits this implements
    pub implemented_by: Vec<RelatedEntity>, // Entities that implement this
}

pub struct RelatedEntity {
    pub entity: CodeEntity,
    pub relationship_type: String,  // "depends_on", "uses", "calls", etc.
    pub metadata: serde_json::Value, // Additional relationship metadata
}
```

---

### RepositoryStats

Statistics about the repository graph.

```rust
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
```

---

## Implementation Notes

### Error Handling

All methods return `Result<T>` where error type is defined as:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Database connection error: {0}")]
    ConnectionError(String),
    
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    
    #[error("Invalid relationship: {0}")]
    InvalidRelationship(String),
    
    #[error("Query execution error: {0}")]
    QueryError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] surrealdb::Error),
}

impl GraphError {
    pub fn to_user_message(&self) -> String {
        match self {
            Self::ConnectionError(_) => 
                "Cannot connect to graph database. Ensure database file is accessible.".to_string(),
            Self::EntityNotFound(name) => 
                format!("Entity '{}' not found in the knowledge graph.", name),
            Self::InvalidRelationship(msg) => 
                format!("Invalid relationship: {}", msg),
            Self::QueryError(_) => 
                "Query execution failed. Please check query syntax.".to_string(),
            Self::ValidationError(msg) => 
                format!("Validation failed: {}", msg),
            Self::DatabaseError(e) => 
                format!("Database error: {}. Check database integrity.", e),
        }
    }
}
```

---

### Transaction Semantics

- **Entity creation**: Atomic per entity
- **Batch operations**: Use transactions for consistency
- **Relationship creation**: Validates node existence before creating edge
- **Cascade deletion**: Uses database-level cascading

---

### Performance Requirements

| Operation | Target Latency | Notes |
|-----------|----------------|-------|
| `create_entity` | <5ms | Single entity |
| `create_entities_batch` | <100ms | 100 entities |
| `find_entities_by_name` | <10ms | Indexed lookup |
| `search_entities_by_name` | <50ms | Fuzzy search with threshold |
| `get_all_relationships` | <20ms | Up to 1000 relationships |
| `traverse_dependencies` | <100ms | Max 3 hops |
| `get_repository_stats` | <200ms | Aggregation query |

---

### Concurrency

- **Read operations**: Thread-safe, concurrent reads allowed
- **Write operations**: Serialize writes per repository
- **Batch writes**: Use connection pooling for parallel batches

---

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_and_get_entity() {
        let client = setup_test_graph_client().await;
        
        let entity = CodeEntity {
            name: "TestEntity".to_string(),
            entity_type: EntityType::Class,
            // ... other fields
        };
        
        let id = client.create_entity(entity.clone()).await.unwrap();
        let retrieved = client.get_entity(&id).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestEntity");
    }
    
    #[tokio::test]
    async fn test_entity_not_found() {
        let client = setup_test_graph_client().await;
        let fake_id = RecordId::from(("code_entity", "nonexistent"));
        
        let result = client.get_entity(&fake_id).await.unwrap();
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_create_dependency_relationship() {
        let client = setup_test_graph_client().await;
        
        let entity1_id = client.create_entity(entity1).await.unwrap();
        let entity2_id = client.create_entity(entity2).await.unwrap();
        
        let rel_id = client.create_dependency(
            &entity1_id, 
            &entity2_id, 
            DependencyType::Import
        ).await.unwrap();
        
        assert!(rel_id.is_valid());
        
        let relationships = client.get_outgoing_relationships(&entity1_id).await.unwrap();
        assert_eq!(relationships.dependencies.len(), 1);
    }
    
    #[tokio::test]
    async fn test_fuzzy_search_entities() {
        let client = setup_test_graph_client().await;
        
        // Create entities with similar names
        client.create_entity(make_entity("Parser")).await.unwrap();
        client.create_entity(make_entity("ParserHelper")).await.unwrap();
        client.create_entity(make_entity("Indexer")).await.unwrap();
        
        let matches = client.search_entities_by_name(
            &repo_id, 
            "Parsr",  // Typo
            0.8       // 80% threshold
        ).await.unwrap();
        
        assert!(matches.len() >= 1);
        assert!(matches[0].entity.name.contains("Parser"));
        assert!(matches[0].confidence > 0.8);
    }
    
    #[tokio::test]
    async fn test_cascade_delete() {
        let client = setup_test_graph_client().await;
        
        let entity1_id = client.create_entity(entity1).await.unwrap();
        let entity2_id = client.create_entity(entity2).await.unwrap();
        
        client.create_dependency(&entity1_id, &entity2_id, DependencyType::Import).await.unwrap();
        
        client.delete_entity_cascade(&entity1_id).await.unwrap();
        
        // Verify entity is deleted
        let result = client.get_entity(&entity1_id).await.unwrap();
        assert!(result.is_none());
        
        // Verify relationship is also deleted
        let relationships = client.get_incoming_relationships(&entity2_id).await.unwrap();
        assert_eq!(relationships.dependents.len(), 0);
    }
    
    #[tokio::test]
    async fn test_batch_create_entities() {
        let client = setup_test_graph_client().await;
        
        let entities: Vec<CodeEntity> = (0..100)
            .map(|i| make_entity(&format!("Entity{}", i)))
            .collect();
        
        let start = std::time::Instant::now();
        let ids = client.create_entities_batch(entities).await.unwrap();
        let duration = start.elapsed();
        
        assert_eq!(ids.len(), 100);
        assert!(duration.as_millis() < 100, "Batch create took too long: {:?}", duration);
    }
}
```

---

### Integration Tests

```rust
#[tokio::test]
async fn test_full_indexing_workflow() {
    let client = setup_test_graph_client().await;
    
    // 1. Create repository
    let repo = Repository {
        name: "test-repo".to_string(),
        path: "/tmp/test".to_string(),
        // ...
    };
    let repo_id = client.create_repository(repo).await.unwrap();
    
    // 2. Create components
    let component_id = client.create_component(/* ... */).await.unwrap();
    
    // 3. Create entities
    let entity_ids = client.create_entities_batch(/* ... */).await.unwrap();
    
    // 4. Create relationships
    for (from, to) in dependencies {
        client.create_dependency(&from, &to, DependencyType::Import).await.unwrap();
    }
    
    // 5. Verify stats
    let stats = client.get_repository_stats(&repo_id).await.unwrap();
    assert_eq!(stats.entity_count, entity_ids.len());
    assert!(stats.relationship_count > 0);
}
```

---

## Contract Validation

### Pre-conditions

- Database must be initialized (schema created)
- Connection must be established before any operation
- Entity references must exist before creating relationships

### Post-conditions

- All created nodes have valid IDs
- Relationships reference existing nodes
- Cascade deletions maintain referential integrity
- Statistics reflect actual graph state

### Invariants

- No orphaned edges (all edges connect existing nodes)
- No dangling references (all foreign keys valid)
- Entity names are non-empty
- Line numbers satisfy `line_start <= line_end`
- Relationship counts are non-negative
