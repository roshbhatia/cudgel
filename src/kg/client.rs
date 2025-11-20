// src/kg/client.rs
//! Knowledge graph database client implementation using PostgreSQL.

use super::{
    CodeEntity, Component, DependencyType, EntityMatch, EntityRelationships, KgError, Repository,
    RepositoryStats, Result,
};
use crate::database::Database;
use async_trait::async_trait;
use std::sync::Arc;

/// Record ID type alias for PostgreSQL primary keys
pub type RecordId = i32;

/// Trait defining knowledge graph database operations
#[async_trait]
pub trait KgClient: Send + Sync {
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

    /// Execute arbitrary SQL query (for complex queries)
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

/// PostgreSQL implementation of the knowledge graph client
pub struct PostgresKgClient {
    db: Arc<Database>,
}

impl PostgresKgClient {
    /// Create a new PostgreSQL KG client
    ///
    /// # Arguments
    /// * `db` - Shared database connection pool
    ///
    /// # Examples
    /// ```no_run
    /// use cudgel::kg::client::PostgresKgClient;
    /// use cudgel::Database;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let db = Database::new(&config).await?;
    ///     let client = PostgresKgClient::new(Arc::new(db));
    ///     Ok(())
    /// }
    /// ```
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get the underlying database connection pool
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }
}

/// Implementation of the KgClient trait for PostgresKgClient
#[async_trait]
impl KgClient for PostgresKgClient {
    // === Repository Operations ===

    /// T041: Create a new repository node
    async fn create_repository(&self, repo: Repository) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        // First, ensure we have a main repository entry (reuse existing or create)
        let main_repo_id: i32 = client
            .query_one(
                "INSERT INTO repositories (repo_path, last_indexed_at) 
                 VALUES ($1, NOW())
                 ON CONFLICT (repo_path) DO UPDATE SET last_indexed_at = NOW()
                 RETURNING id",
                &[&repo.path],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create main repository: {}", e)))?
            .get(0);

        // Now create the KG repository node
        let row = client
            .query_one(
                "INSERT INTO kg_repositories (repository_id, path, name, summary) 
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (path) DO UPDATE 
                 SET name = EXCLUDED.name, summary = EXCLUDED.summary
                 RETURNING id",
                &[&main_repo_id, &repo.path, &repo.name, &repo.summary],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create kg repository: {}", e)))?;

        Ok(row.get(0))
    }

    /// T042: Get repository by path
    async fn get_repository_by_path(&self, path: &str) -> Result<Option<Repository>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let row = client
            .query_opt(
                "SELECT id, path, name, summary, created_at, updated_at 
                 FROM kg_repositories 
                 WHERE path = $1",
                &[&path],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to get repository: {}", e)))?;

        Ok(row.map(|r| Repository {
            id: r.get(0),
            path: r.get(1),
            name: r.get(2),
            summary: r.get(3),
            created_at: r.get(4),
            updated_at: r.get(5),
        }))
    }

    /// T043: Update repository summary
    async fn update_repository_summary(&self, repo_id: &RecordId, summary: String) -> Result<()> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        client
            .execute(
                "UPDATE kg_repositories SET summary = $1, updated_at = NOW() WHERE id = $2",
                &[&summary, &repo_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to update repository summary: {}", e)))?;

        Ok(())
    }

    // === Component Operations ===

    /// T044: Create a new component node
    async fn create_component(&self, component: Component) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let component_type_str = format!("{:?}", component.component_type).to_lowercase();

        let row = client
            .query_one(
                "INSERT INTO kg_components (kg_repository_id, name, path, component_type, summary) 
                 VALUES ($1, $2, $3, $4, $5)
                 RETURNING id",
                &[
                    &component.repository_id,
                    &component.name,
                    &component.path,
                    &component_type_str,
                    &component.summary,
                ],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create component: {}", e)))?;

        Ok(row.get(0))
    }

    /// T045: Get all components in a repository
    async fn get_components(&self, repo_id: &RecordId) -> Result<Vec<Component>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let rows = client
            .query(
                "SELECT id, kg_repository_id, name, path, component_type, summary, created_at, updated_at 
                 FROM kg_components 
                 WHERE kg_repository_id = $1
                 ORDER BY name",
                &[repo_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to get components: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let component_type_str: String = r.get(4);
                let component_type = match component_type_str.as_str() {
                    "module" => super::ComponentType::Module,
                    "package" => super::ComponentType::Package,
                    _ => super::ComponentType::Directory,
                };

                Component {
                    id: r.get(0),
                    repository_id: r.get(1),
                    name: r.get(2),
                    path: r.get(3),
                    component_type,
                    summary: r.get(5),
                    created_at: r.get(6),
                    updated_at: r.get(7),
                }
            })
            .collect())
    }

    /// T046: Update component summary
    async fn update_component_summary(&self, component_id: &RecordId, summary: String) -> Result<()> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        client
            .execute(
                "UPDATE kg_components SET summary = $1, updated_at = NOW() WHERE id = $2",
                &[&summary, component_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to update component summary: {}", e)))?;

        Ok(())
    }

    // === Entity Operations ===

    /// T047: Create a new code entity node
    async fn create_entity(&self, entity: CodeEntity) -> Result<RecordId> {
        let client = self.db.get_client().await.map_err(|e| {
            KgError::Database(format!("Failed to get database client: {}", e))
        })?;

        let entity_type_str = format!("{:?}", entity.entity_type).to_lowercase();
        let visibility_str = format!("{:?}", entity.visibility).to_lowercase();

        let row = client
            .query_one(
                "INSERT INTO kg_entities (
                    kg_component_id, name, entity_type, file_path, 
                    line_start, line_end, visibility, signature, doc_comment, 
                    language, summary
                 ) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 RETURNING id",
                &[
                    &entity.component_id,
                    &entity.name,
                    &entity_type_str,
                    &entity.file_path,
                    &(entity.line_start as i32),
                    &(entity.line_end as i32),
                    &visibility_str,
                    &entity.metadata.signature,
                    &entity.metadata.doc_comment,
                    &entity.metadata.language,
                    &entity.summary,
                ],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create entity: {}", e)))?;

        Ok(row.get(0))
    }

    /// T048: Batch create multiple entities with transaction batching
    async fn create_entities_batch(&self, entities: Vec<CodeEntity>) -> Result<Vec<RecordId>> {
        let mut client = self.db.get_client().await.map_err(|e| {
            KgError::Database(format!("Failed to get database client: {}", e))
        })?;

        // Use a transaction for batch insert
        let transaction = client.transaction().await.map_err(|e| {
            KgError::Database(format!("Failed to start transaction: {}", e))
        })?;

        let mut ids = Vec::with_capacity(entities.len());

        for entity in entities {
            let entity_type_str = format!("{:?}", entity.entity_type).to_lowercase();
            let visibility_str = format!("{:?}", entity.visibility).to_lowercase();

            let row = transaction
                .query_one(
                    "INSERT INTO kg_entities (
                        kg_component_id, name, entity_type, file_path, 
                        line_start, line_end, visibility, signature, doc_comment, 
                        language, summary
                     ) 
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                     RETURNING id",
                    &[
                        &entity.component_id,
                        &entity.name,
                        &entity_type_str,
                        &entity.file_path,
                        &(entity.line_start as i32),
                        &(entity.line_end as i32),
                        &visibility_str,
                        &entity.metadata.signature,
                        &entity.metadata.doc_comment,
                        &entity.metadata.language,
                        &entity.summary,
                    ],
                )
                .await
                .map_err(|e| KgError::Database(format!("Failed to insert entity: {}", e)))?;

            ids.push(row.get(0));
        }

        transaction.commit().await.map_err(|e| {
            KgError::Database(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(ids)
    }

    async fn get_entity(&self, _entity_id: &RecordId) -> Result<Option<CodeEntity>> {
        todo!("T044: Implement get_entity")
    }

    async fn find_entities_by_name(
        &self,
        _repo_id: &RecordId,
        _name: &str,
    ) -> Result<Vec<CodeEntity>> {
        todo!("T044: Implement find_entities_by_name")
    }

    async fn search_entities_by_name(
        &self,
        _repo_id: &RecordId,
        _pattern: &str,
        _threshold: f64,
    ) -> Result<Vec<EntityMatch>> {
        todo!("T050: Implement search_entities_by_name")
    }

    async fn get_entities_by_file(
        &self,
        _repo_id: &RecordId,
        _file_path: &str,
    ) -> Result<Vec<CodeEntity>> {
        todo!("T044: Implement get_entities_by_file")
    }

    async fn get_all_entity_names(&self, _repo_id: &RecordId) -> Result<Vec<String>> {
        todo!("T050: Implement get_all_entity_names")
    }

    async fn update_entity_summary(&self, _entity_id: &RecordId, _summary: String) -> Result<()> {
        todo!("T068: Implement update_entity_summary")
    }

    async fn delete_entity_cascade(&self, _entity_id: &RecordId) -> Result<()> {
        todo!("T163: Implement delete_entity_cascade")
    }

    // === Relationship Operations ===

    async fn create_dependency(
        &self,
        _from: &RecordId,
        _to: &RecordId,
        _dep_type: DependencyType,
    ) -> Result<RecordId> {
        todo!("T099: Implement create_dependency")
    }

    async fn create_uses(
        &self,
        _from: &RecordId,
        _to: &RecordId,
        _context: String,
    ) -> Result<RecordId> {
        todo!("T099: Implement create_uses")
    }

    async fn create_contains(&self, _from: &RecordId, _to: &RecordId) -> Result<RecordId> {
        todo!("T099: Implement create_contains")
    }

    async fn create_implements(&self, _from: &RecordId, _to: &RecordId) -> Result<RecordId> {
        todo!("T099: Implement create_implements")
    }

    async fn create_calls(
        &self,
        _from: &RecordId,
        _to: &RecordId,
        _call_count: usize,
    ) -> Result<RecordId> {
        todo!("T099: Implement create_calls")
    }

    async fn get_outgoing_relationships(
        &self,
        _entity_id: &RecordId,
    ) -> Result<EntityRelationships> {
        todo!("T100: Implement get_outgoing_relationships")
    }

    async fn get_incoming_relationships(
        &self,
        _entity_id: &RecordId,
    ) -> Result<EntityRelationships> {
        todo!("T100: Implement get_incoming_relationships")
    }

    async fn get_all_relationships(&self, _entity_id: &RecordId) -> Result<EntityRelationships> {
        todo!("T100: Implement get_all_relationships")
    }

    async fn traverse_dependencies(
        &self,
        _entity_id: &RecordId,
        _max_depth: usize,
    ) -> Result<Vec<CodeEntity>> {
        todo!("T118: Implement traverse_dependencies")
    }

    // === Query Operations ===

    async fn execute_query(&self, _query: &str) -> Result<Vec<serde_json::Value>> {
        todo!("T147: Implement execute_query via Database wrapper")
    }

    async fn get_repository_stats(&self, _repo_id: &RecordId) -> Result<RepositoryStats> {
        todo!("T147: Implement get_repository_stats")
    }

    // === Maintenance Operations ===

    async fn initialize_schema(&self) -> Result<()> {
        self.db
            .init_kg_schema()
            .await
            .map_err(|e| KgError::Database(format!("Failed to initialize schema: {}", e)))
    }

    async fn is_schema_initialized(&self) -> Result<bool> {
        self.db
            .is_kg_schema_initialized()
            .await
            .map_err(|e| KgError::Database(format!("Failed to check schema: {}", e)))
    }

    async fn optimize(&self) -> Result<()> {
        // PostgreSQL VACUUM - would need to be done via psql or direct SQL
        // For now, this is a no-op as VACUUM needs superuser privileges
        // Autovacuum handles this automatically in most cases
        Ok(())
    }
}
