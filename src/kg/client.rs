// src/kg/client.rs
//! Knowledge graph database client implementation using PostgreSQL.

use super::{
    CodeEntity, Component, DependencyType, EntityMatch, EntityMetadata, EntityRelationships,
    EntityType, KgError, RelatedEntity, Repository, RepositoryStats, Result, Visibility,
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
    async fn update_component_summary(
        &self,
        component_id: &RecordId,
        summary: String,
    ) -> Result<()>;

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
    async fn create_uses(
        &self,
        from: &RecordId,
        to: &RecordId,
        context: String,
    ) -> Result<RecordId>;

    /// Create a CONTAINS relationship
    async fn create_contains(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;

    /// Create an IMPLEMENTS relationship
    async fn create_implements(&self, from: &RecordId, to: &RecordId) -> Result<RecordId>;

    /// Create a CALLS relationship
    async fn create_calls(
        &self,
        from: &RecordId,
        to: &RecordId,
        call_count: usize,
    ) -> Result<RecordId>;

    /// Get outgoing relationships for an entity
    async fn get_outgoing_relationships(&self, entity_id: &RecordId)
        -> Result<EntityRelationships>;

    /// Get incoming relationships for an entity
    async fn get_incoming_relationships(&self, entity_id: &RecordId)
        -> Result<EntityRelationships>;

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
    /// use cudgel::database::Database;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = cudgel::config::Config::local()?;
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

    /// Helper to convert a database row to a CodeEntity
    ///
    /// # Arguments
    /// * `row` - The database row
    /// * `offset` - Column offset where entity fields start
    fn row_to_entity(&self, row: &tokio_postgres::Row, offset: usize) -> Result<CodeEntity> {
        use chrono::{DateTime, Utc};

        let id: i32 = row.get(offset);
        let name: String = row.get(offset + 1);
        let entity_type_str: String = row.get(offset + 2);
        let file_path: String = row.get(offset + 3);
        let line_start: i32 = row.get(offset + 4);
        let line_end: i32 = row.get(offset + 5);
        let visibility_str: String = row.get(offset + 6);
        let signature: Option<String> = row.get(offset + 7);
        let doc_comment: Option<String> = row.get(offset + 8);
        let language: String = row.get(offset + 9);
        let summary: Option<String> = row.get(offset + 10);
        let created_at: DateTime<Utc> = row.get(offset + 11);
        let updated_at: DateTime<Utc> = row.get(offset + 12);
        let component_id: i32 = row.get(offset + 13);

        let entity_type = match entity_type_str.as_str() {
            "function" => EntityType::Function,
            "class" => EntityType::Class,
            "struct" => EntityType::Struct,
            "enum" => EntityType::Enum,
            "interface" => EntityType::Interface,
            "trait" => EntityType::Trait,
            "method" => EntityType::Method,
            "constant" => EntityType::Constant,
            "variable" => EntityType::Variable,
            _ => {
                return Err(KgError::InvalidInput(format!(
                    "Unknown entity type: {}",
                    entity_type_str
                )))
            }
        };

        let visibility = match visibility_str.as_str() {
            "public" => Visibility::Public,
            "private" => Visibility::Private,
            "protected" => Visibility::Protected,
            "internal" => Visibility::Internal,
            _ => {
                return Err(KgError::InvalidInput(format!(
                    "Unknown visibility: {}",
                    visibility_str
                )))
            }
        };

        Ok(CodeEntity {
            id,
            component_id,
            name,
            entity_type,
            file_path,
            line_start: line_start as u32,
            line_end: line_end as u32,
            visibility,
            metadata: EntityMetadata {
                signature,
                doc_comment,
                language,
            },
            summary,
            created_at,
            updated_at,
        })
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
            .map_err(|e| {
                KgError::Database(format!("Failed to update repository summary: {}", e))
            })?;

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
    async fn update_component_summary(
        &self,
        component_id: &RecordId,
        summary: String,
    ) -> Result<()> {
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
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let entity_type_str = format!("{:?}", entity.entity_type).to_lowercase();
        let visibility_str = format!("{:?}", entity.visibility).to_lowercase();
        let metadata_json = serde_json::to_value(&entity.metadata)
            .map_err(|e| KgError::Database(format!("Failed to serialize metadata: {}", e)))?;

        let row = client
            .query_one(
                "INSERT INTO kg_entities (
                    kg_component_id, name, entity_type, file_path, 
                    line_start, line_end, visibility, signature, 
                    doc_comment, language, summary, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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
                    &metadata_json,
                ],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create entity: {}", e)))?;

        Ok(row.get(0))
    }

    /// T048: Batch create multiple entities with transaction batching
    async fn create_entities_batch(&self, entities: Vec<CodeEntity>) -> Result<Vec<RecordId>> {
        if entities.is_empty() {
            return Ok(vec![]);
        }

        let mut client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let transaction = client
            .transaction()
            .await
            .map_err(|e| KgError::Database(format!("Failed to start transaction: {}", e)))?;

        let mut ids = Vec::with_capacity(entities.len());

        for entity in entities {
            let entity_type_str = format!("{:?}", entity.entity_type).to_lowercase();
            let visibility_str = format!("{:?}", entity.visibility).to_lowercase();
            let metadata_json = serde_json::to_value(&entity.metadata)
                .map_err(|e| KgError::Database(format!("Failed to serialize metadata: {}", e)))?;

            let row = transaction
                .query_one(
                    "INSERT INTO kg_entities (
                        kg_component_id, name, entity_type, file_path, 
                        line_start, line_end, visibility, signature, 
                        doc_comment, language, summary, metadata
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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
                        &metadata_json,
                    ],
                )
                .await
                .map_err(|e| {
                    KgError::Database(format!("Failed to create entity in batch: {}", e))
                })?;

            ids.push(row.get(0));
        }

        transaction
            .commit()
            .await
            .map_err(|e| KgError::Database(format!("Failed to commit batch transaction: {}", e)))?;

        Ok(ids)
    }

    async fn get_entity(&self, entity_id: &RecordId) -> Result<Option<CodeEntity>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let row = client
            .query_opt(
                "SELECT id, kg_component_id, name, entity_type, file_path, 
                       line_start, line_end, visibility, signature, 
                       doc_comment, language, summary, metadata, created_at, updated_at
                FROM kg_entities 
                WHERE id = $1",
                &[entity_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to get entity: {}", e)))?;

        Ok(row.map(|r| {
            let entity_type_str: String = r.get(3);
            let visibility_str: String = r.get(7);
            let metadata_json: serde_json::Value = r.get(12);

            let entity_type = match entity_type_str.as_str() {
                "function" => super::EntityType::Function,
                "class" => super::EntityType::Class,
                "struct" => super::EntityType::Struct,
                "enum" => super::EntityType::Enum,
                "interface" => super::EntityType::Interface,
                "trait" => super::EntityType::Trait,
                "method" => super::EntityType::Method,
                "constant" => super::EntityType::Constant,
                "variable" => super::EntityType::Variable,
                _ => super::EntityType::Function, // Default fallback
            };

            let visibility = match visibility_str.as_str() {
                "public" => super::Visibility::Public,
                "private" => super::Visibility::Private,
                "protected" => super::Visibility::Protected,
                "internal" => super::Visibility::Internal,
                _ => super::Visibility::Private, // Default fallback
            };

            let metadata: super::EntityMetadata =
                serde_json::from_value(metadata_json).unwrap_or_default();

            CodeEntity {
                id: r.get(0),
                component_id: r.get(1),
                name: r.get(2),
                entity_type,
                file_path: r.get(4),
                line_start: r.get::<_, i32>(5) as u32,
                line_end: r.get::<_, i32>(6) as u32,
                visibility,
                metadata,
                summary: r.get(11),
                created_at: r.get(13),
                updated_at: r.get(14),
            }
        }))
    }

    async fn find_entities_by_name(
        &self,
        repo_id: &RecordId,
        name: &str,
    ) -> Result<Vec<CodeEntity>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let rows = client
            .query(
                "SELECT e.id, e.kg_component_id, e.name, e.entity_type, e.file_path, 
                       e.line_start, e.line_end, e.visibility, e.signature, 
                       e.doc_comment, e.language, e.summary, e.metadata, e.created_at, e.updated_at
                FROM kg_entities e
                JOIN kg_components c ON e.kg_component_id = c.id
                JOIN kg_repositories r ON c.kg_repository_id = r.id
                WHERE r.repository_id = $1 AND e.name = $2
                ORDER BY e.name",
                &[repo_id, &name],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to find entities by name: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let entity_type_str: String = r.get(3);
                let visibility_str: String = r.get(7);
                let metadata_json: serde_json::Value = r.get(12);

                let entity_type = match entity_type_str.as_str() {
                    "function" => super::EntityType::Function,
                    "class" => super::EntityType::Class,
                    "struct" => super::EntityType::Struct,
                    "enum" => super::EntityType::Enum,
                    "interface" => super::EntityType::Interface,
                    "trait" => super::EntityType::Trait,
                    "method" => super::EntityType::Method,
                    "constant" => super::EntityType::Constant,
                    "variable" => super::EntityType::Variable,
                    _ => super::EntityType::Function,
                };

                let visibility = match visibility_str.as_str() {
                    "public" => super::Visibility::Public,
                    "private" => super::Visibility::Private,
                    "protected" => super::Visibility::Protected,
                    "internal" => super::Visibility::Internal,
                    _ => super::Visibility::Private,
                };

                let metadata: super::EntityMetadata =
                    serde_json::from_value(metadata_json).unwrap_or_default();

                CodeEntity {
                    id: r.get(0),
                    component_id: r.get(1),
                    name: r.get(2),
                    entity_type,
                    file_path: r.get(4),
                    line_start: r.get::<_, i32>(5) as u32,
                    line_end: r.get::<_, i32>(6) as u32,
                    visibility,
                    metadata,
                    summary: r.get(11),
                    created_at: r.get(13),
                    updated_at: r.get(14),
                }
            })
            .collect())
    }

    async fn search_entities_by_name(
        &self,
        repo_id: &RecordId,
        pattern: &str,
        threshold: f64,
    ) -> Result<Vec<EntityMatch>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        // Use PostgreSQL's similarity search with pattern matching
        let rows = client
            .query(
                "SELECT e.id, e.kg_component_id, e.name, e.entity_type, e.file_path, 
                       e.line_start, e.line_end, e.visibility, e.signature, 
                       e.doc_comment, e.language, e.summary, e.metadata, e.created_at, e.updated_at,
                       CASE 
                           WHEN e.name ILIKE $1 THEN 1.0
                           WHEN e.name ILIKE CONCAT('%', $1, '%') THEN 0.8
                           ELSE 0.5
                       END as confidence
                FROM kg_entities e
                JOIN kg_components c ON e.kg_component_id = c.id
                JOIN kg_repositories r ON c.kg_repository_id = r.id
                WHERE r.repository_id = $2 
                  AND (e.name ILIKE CONCAT('%', $1, '%') OR e.name % $1)
                  AND CASE 
                      WHEN e.name ILIKE $1 THEN 1.0
                      WHEN e.name ILIKE CONCAT('%', $1, '%') THEN 0.8
                      ELSE 0.5
                  END >= $3
                ORDER BY confidence DESC, e.name
                LIMIT 50",
                &[&pattern, repo_id, &threshold],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to search entities: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let entity_type_str: String = r.get(3);
                let visibility_str: String = r.get(7);
                let metadata_json: serde_json::Value = r.get(12);
                let confidence: f64 = r.get(14);

                let entity_type = match entity_type_str.as_str() {
                    "function" => super::EntityType::Function,
                    "class" => super::EntityType::Class,
                    "struct" => super::EntityType::Struct,
                    "enum" => super::EntityType::Enum,
                    "interface" => super::EntityType::Interface,
                    "trait" => super::EntityType::Trait,
                    "method" => super::EntityType::Method,
                    "constant" => super::EntityType::Constant,
                    "variable" => super::EntityType::Variable,
                    _ => super::EntityType::Function,
                };

                let visibility = match visibility_str.as_str() {
                    "public" => super::Visibility::Public,
                    "private" => super::Visibility::Private,
                    "protected" => super::Visibility::Protected,
                    "internal" => super::Visibility::Internal,
                    _ => super::Visibility::Private,
                };

                let metadata: super::EntityMetadata =
                    serde_json::from_value(metadata_json).unwrap_or_default();

                let entity = CodeEntity {
                    id: r.get(0),
                    component_id: r.get(1),
                    name: r.get(2),
                    entity_type,
                    file_path: r.get(4),
                    line_start: r.get::<_, i32>(5) as u32,
                    line_end: r.get::<_, i32>(6) as u32,
                    visibility,
                    metadata,
                    summary: r.get(11),
                    created_at: r.get(13),
                    updated_at: r.get(14),
                };

                super::EntityMatch { entity, confidence }
            })
            .collect())
    }

    async fn get_entities_by_file(
        &self,
        repo_id: &RecordId,
        file_path: &str,
    ) -> Result<Vec<CodeEntity>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let rows = client
            .query(
                "SELECT e.id, e.kg_component_id, e.name, e.entity_type, e.file_path, 
                       e.line_start, e.line_end, e.visibility, e.signature, 
                       e.doc_comment, e.language, e.summary, e.metadata, e.created_at, e.updated_at
                FROM kg_entities e
                JOIN kg_components c ON e.kg_component_id = c.id
                JOIN kg_repositories r ON c.kg_repository_id = r.id
                WHERE r.repository_id = $1 AND e.file_path = $2
                ORDER BY e.line_start",
                &[repo_id, &file_path],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to get entities by file: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let entity_type_str: String = r.get(3);
                let visibility_str: String = r.get(7);
                let metadata_json: serde_json::Value = r.get(12);

                let entity_type = match entity_type_str.as_str() {
                    "function" => super::EntityType::Function,
                    "class" => super::EntityType::Class,
                    "struct" => super::EntityType::Struct,
                    "enum" => super::EntityType::Enum,
                    "interface" => super::EntityType::Interface,
                    "trait" => super::EntityType::Trait,
                    "method" => super::EntityType::Method,
                    "constant" => super::EntityType::Constant,
                    "variable" => super::EntityType::Variable,
                    _ => super::EntityType::Function,
                };

                let visibility = match visibility_str.as_str() {
                    "public" => super::Visibility::Public,
                    "private" => super::Visibility::Private,
                    "protected" => super::Visibility::Protected,
                    "internal" => super::Visibility::Internal,
                    _ => super::Visibility::Private,
                };

                let metadata: super::EntityMetadata =
                    serde_json::from_value(metadata_json).unwrap_or_default();

                CodeEntity {
                    id: r.get(0),
                    component_id: r.get(1),
                    name: r.get(2),
                    entity_type,
                    file_path: r.get(4),
                    line_start: r.get::<_, i32>(5) as u32,
                    line_end: r.get::<_, i32>(6) as u32,
                    visibility,
                    metadata,
                    summary: r.get(11),
                    created_at: r.get(13),
                    updated_at: r.get(14),
                }
            })
            .collect())
    }

    async fn get_all_entity_names(&self, repo_id: &RecordId) -> Result<Vec<String>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let rows = client
            .query(
                "SELECT DISTINCT e.name
                FROM kg_entities e
                JOIN kg_components c ON e.kg_component_id = c.id
                JOIN kg_repositories r ON c.kg_repository_id = r.id
                WHERE r.repository_id = $1
                ORDER BY e.name",
                &[repo_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to get entity names: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.get(0)).collect())
    }

    async fn update_entity_summary(&self, entity_id: &RecordId, summary: String) -> Result<()> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        client
            .execute(
                "UPDATE kg_entities SET summary = $1, updated_at = NOW() WHERE id = $2",
                &[&summary, entity_id],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to update entity summary: {}", e)))?;

        Ok(())
    }

    async fn delete_entity_cascade(&self, entity_id: &RecordId) -> Result<()> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        // The foreign key constraints with ON DELETE CASCADE will handle cleanup
        client
            .execute("DELETE FROM kg_entities WHERE id = $1", &[entity_id])
            .await
            .map_err(|e| KgError::Database(format!("Failed to delete entity: {}", e)))?;

        Ok(())
    }

    // === Relationship Operations ===

    /// T090: Create a DEPENDS_ON relationship
    async fn create_dependency(
        &self,
        from: &RecordId,
        to: &RecordId,
        dep_type: DependencyType,
    ) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let dep_type_str = match dep_type {
            DependencyType::Import => "import",
            DependencyType::Inheritance => "inheritance",
            DependencyType::Composition => "composition",
            DependencyType::Association => "association",
        };

        let row = client
            .query_one(
                "INSERT INTO kg_relationships 
                 (from_entity_id, to_entity_id, relationship_type, dep_type)
                 VALUES ($1, $2, 'depends_on', $3)
                 ON CONFLICT (from_entity_id, to_entity_id, relationship_type) 
                 DO UPDATE SET dep_type = EXCLUDED.dep_type
                 RETURNING id",
                &[from, to, &dep_type_str],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to create dependency relationship: {}", e))
            })?;

        Ok(row.get(0))
    }

    /// T091: Create a USES relationship
    async fn create_uses(
        &self,
        from: &RecordId,
        to: &RecordId,
        context: String,
    ) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let row = client
            .query_one(
                "INSERT INTO kg_relationships 
                 (from_entity_id, to_entity_id, relationship_type, context)
                 VALUES ($1, $2, 'uses', $3)
                 ON CONFLICT (from_entity_id, to_entity_id, relationship_type) 
                 DO UPDATE SET context = EXCLUDED.context
                 RETURNING id",
                &[from, to, &context],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to create uses relationship: {}", e)))?;

        Ok(row.get(0))
    }

    /// T092: Create a CONTAINS relationship
    async fn create_contains(&self, from: &RecordId, to: &RecordId) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let row = client
            .query_one(
                "INSERT INTO kg_relationships 
                 (from_entity_id, to_entity_id, relationship_type)
                 VALUES ($1, $2, 'contains')
                 ON CONFLICT (from_entity_id, to_entity_id, relationship_type) 
                 DO NOTHING
                 RETURNING id",
                &[from, to],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to create contains relationship: {}", e))
            })?;

        Ok(row.get(0))
    }

    /// T093: Create an IMPLEMENTS relationship
    async fn create_implements(&self, from: &RecordId, to: &RecordId) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let row = client
            .query_one(
                "INSERT INTO kg_relationships 
                 (from_entity_id, to_entity_id, relationship_type)
                 VALUES ($1, $2, 'implements')
                 ON CONFLICT (from_entity_id, to_entity_id, relationship_type) 
                 DO NOTHING
                 RETURNING id",
                &[from, to],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to create implements relationship: {}", e))
            })?;

        Ok(row.get(0))
    }

    /// T094: Create a CALLS relationship
    async fn create_calls(
        &self,
        from: &RecordId,
        to: &RecordId,
        call_count: usize,
    ) -> Result<RecordId> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let call_count_i32 = call_count as i32;

        let row = client
            .query_one(
                "INSERT INTO kg_relationships 
                 (from_entity_id, to_entity_id, relationship_type, call_count)
                 VALUES ($1, $2, 'calls', $3)
                 ON CONFLICT (from_entity_id, to_entity_id, relationship_type) 
                 DO UPDATE SET call_count = EXCLUDED.call_count
                 RETURNING id",
                &[from, to, &call_count_i32],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to create calls relationship: {}", e))
            })?;

        Ok(row.get(0))
    }

    /// T095: Get outgoing relationships for an entity (what this entity depends on/uses/calls)
    async fn get_outgoing_relationships(
        &self,
        entity_id: &RecordId,
    ) -> Result<EntityRelationships> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let mut relationships = EntityRelationships::default();

        // Get all outgoing relationships
        let rows = client
            .query(
                "SELECT r.relationship_type, r.dep_type, r.context, r.call_count, r.metadata,
                        e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                        e.visibility, e.signature, e.doc_comment, e.language, e.summary,
                        e.created_at, e.updated_at, e.kg_component_id
                 FROM kg_relationships r
                 JOIN kg_entities e ON r.to_entity_id = e.id
                 WHERE r.from_entity_id = $1",
                &[entity_id],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to get outgoing relationships: {}", e))
            })?;

        for row in rows {
            let relationship_type: String = row.get(0);
            let entity = self.row_to_entity(&row, 5)?;

            let metadata = row
                .get::<_, Option<serde_json::Value>>(4)
                .unwrap_or_else(|| serde_json::json!({}));

            let related = RelatedEntity {
                entity,
                relationship_type: relationship_type.clone(),
                metadata,
            };

            match relationship_type.as_str() {
                "depends_on" => relationships.dependencies.push(related),
                "uses" => relationships.uses.push(related),
                "calls" => relationships.calls.push(related),
                "implements" => relationships.implements.push(related),
                _ => {}
            }
        }

        Ok(relationships)
    }

    /// T096: Get incoming relationships for an entity (what depends on/uses/calls this entity)
    async fn get_incoming_relationships(
        &self,
        entity_id: &RecordId,
    ) -> Result<EntityRelationships> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        let mut relationships = EntityRelationships::default();

        // Get all incoming relationships
        let rows = client
            .query(
                "SELECT r.relationship_type, r.dep_type, r.context, r.call_count, r.metadata,
                        e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                        e.visibility, e.signature, e.doc_comment, e.language, e.summary,
                        e.created_at, e.updated_at, e.kg_component_id
                 FROM kg_relationships r
                 JOIN kg_entities e ON r.from_entity_id = e.id
                 WHERE r.to_entity_id = $1",
                &[entity_id],
            )
            .await
            .map_err(|e| {
                KgError::Database(format!("Failed to get incoming relationships: {}", e))
            })?;

        for row in rows {
            let relationship_type: String = row.get(0);
            let entity = self.row_to_entity(&row, 5)?;

            let metadata = row
                .get::<_, Option<serde_json::Value>>(4)
                .unwrap_or_else(|| serde_json::json!({}));

            let related = RelatedEntity {
                entity,
                relationship_type: relationship_type.clone(),
                metadata,
            };

            match relationship_type.as_str() {
                "depends_on" => relationships.dependents.push(related),
                "uses" => relationships.used_by.push(related),
                "calls" => relationships.called_by.push(related),
                "implements" => relationships.implemented_by.push(related),
                _ => {}
            }
        }

        Ok(relationships)
    }

    /// T097: Get all relationships for an entity (both incoming and outgoing)
    async fn get_all_relationships(&self, entity_id: &RecordId) -> Result<EntityRelationships> {
        let outgoing = self.get_outgoing_relationships(entity_id).await?;
        let incoming = self.get_incoming_relationships(entity_id).await?;

        Ok(EntityRelationships {
            dependencies: outgoing.dependencies,
            dependents: incoming.dependents,
            uses: outgoing.uses,
            used_by: incoming.used_by,
            calls: outgoing.calls,
            called_by: incoming.called_by,
            implements: outgoing.implements,
            implemented_by: incoming.implemented_by,
        })
    }

    /// T098: Traverse dependencies recursively up to max_depth
    async fn traverse_dependencies(
        &self,
        entity_id: &RecordId,
        max_depth: usize,
    ) -> Result<Vec<CodeEntity>> {
        let client = self
            .db
            .get_pool_client()
            .await
            .map_err(|e| KgError::Database(format!("Failed to get database client: {}", e)))?;

        // Use recursive CTE to traverse dependency graph
        let max_depth_i32 = max_depth as i32;
        let rows = client
            .query(
                "WITH RECURSIVE dependency_tree AS (
                    -- Base case: direct dependencies
                    SELECT 
                        r.to_entity_id as entity_id,
                        1 as depth
                    FROM kg_relationships r
                    WHERE r.from_entity_id = $1
                      AND r.relationship_type = 'depends_on'
                    
                    UNION
                    
                    -- Recursive case: transitive dependencies
                    SELECT 
                        r.to_entity_id as entity_id,
                        dt.depth + 1
                    FROM kg_relationships r
                    INNER JOIN dependency_tree dt ON r.from_entity_id = dt.entity_id
                    WHERE r.relationship_type = 'depends_on'
                      AND dt.depth < $2
                )
                SELECT DISTINCT
                    e.id, e.name, e.entity_type, e.file_path, e.line_start, e.line_end,
                    e.visibility, e.signature, e.doc_comment, e.language, e.summary,
                    e.created_at, e.updated_at, e.kg_component_id
                FROM dependency_tree dt
                JOIN kg_entities e ON dt.entity_id = e.id
                ORDER BY dt.depth, e.name",
                &[entity_id, &max_depth_i32],
            )
            .await
            .map_err(|e| KgError::Database(format!("Failed to traverse dependencies: {}", e)))?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(self.row_to_entity(&row, 0)?);
        }

        Ok(entities)
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
