// src/kg/schema.rs
//! Schema management for the knowledge graph stored in PostgreSQL.
//!
//! The knowledge graph extends the existing PostgreSQL schema with additional tables
//! for repository/component/entity hierarchies and LLM-generated summaries.

use super::{KgError, Result};
use tokio_postgres::Client;

/// SQL schema for knowledge graph tables
///
/// Creates tables for:
/// - kg_repositories: Top-level repository nodes with architecture summaries
/// - kg_components: Module/package level nodes
/// - kg_entities: Code entity level nodes (classes, functions, etc.)
/// - kg_relationships: Edges between entities
pub const KG_SCHEMA: &str = r#"
-- Knowledge Graph: Repository nodes
CREATE TABLE IF NOT EXISTS kg_repositories (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    summary TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_kg_repositories_repo_id ON kg_repositories(repository_id);
CREATE INDEX IF NOT EXISTS idx_kg_repositories_path ON kg_repositories(path);

-- Knowledge Graph: Component nodes (modules, packages)
CREATE TABLE IF NOT EXISTS kg_components (
    id SERIAL PRIMARY KEY,
    kg_repository_id INTEGER NOT NULL REFERENCES kg_repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    component_type TEXT NOT NULL, -- 'module', 'package', 'directory'
    summary TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_kg_components_repo ON kg_components(kg_repository_id);
CREATE INDEX IF NOT EXISTS idx_kg_components_path ON kg_components(path);
CREATE INDEX IF NOT EXISTS idx_kg_components_name ON kg_components(name);

-- Knowledge Graph: Code entity nodes (classes, functions, etc.)
CREATE TABLE IF NOT EXISTS kg_entities (
    id SERIAL PRIMARY KEY,
    kg_component_id INTEGER NOT NULL REFERENCES kg_components(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL, -- 'function', 'class', 'struct', 'enum', etc.
    file_path TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    visibility TEXT NOT NULL, -- 'public', 'private', 'protected', 'internal'
    signature TEXT,
    doc_comment TEXT,
    language TEXT NOT NULL,
    summary TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_kg_entities_component ON kg_entities(kg_component_id);
CREATE INDEX IF NOT EXISTS idx_kg_entities_name ON kg_entities(name);
CREATE INDEX IF NOT EXISTS idx_kg_entities_file ON kg_entities(file_path);
CREATE INDEX IF NOT EXISTS idx_kg_entities_type ON kg_entities(entity_type);

-- Knowledge Graph: Relationships (edges)
CREATE TABLE IF NOT EXISTS kg_relationships (
    id SERIAL PRIMARY KEY,
    from_entity_id INTEGER NOT NULL REFERENCES kg_entities(id) ON DELETE CASCADE,
    to_entity_id INTEGER NOT NULL REFERENCES kg_entities(id) ON DELETE CASCADE,
    relationship_type TEXT NOT NULL, -- 'depends_on', 'uses', 'calls', 'implements', 'extends'
    dep_type TEXT, -- For 'depends_on': 'import', 'inheritance', 'composition', 'association'
    context TEXT, -- Additional context (e.g., function call site)
    call_count INTEGER, -- For 'calls' relationships
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Prevent duplicate relationships
    UNIQUE(from_entity_id, to_entity_id, relationship_type)
);

CREATE INDEX IF NOT EXISTS idx_kg_relationships_from ON kg_relationships(from_entity_id);
CREATE INDEX IF NOT EXISTS idx_kg_relationships_to ON kg_relationships(to_entity_id);
CREATE INDEX IF NOT EXISTS idx_kg_relationships_type ON kg_relationships(relationship_type);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_kg_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_kg_repositories_updated_at BEFORE UPDATE ON kg_repositories
    FOR EACH ROW EXECUTE FUNCTION update_kg_updated_at_column();

CREATE TRIGGER update_kg_components_updated_at BEFORE UPDATE ON kg_components
    FOR EACH ROW EXECUTE FUNCTION update_kg_updated_at_column();

CREATE TRIGGER update_kg_entities_updated_at BEFORE UPDATE ON kg_entities
    FOR EACH ROW EXECUTE FUNCTION update_kg_updated_at_column();
"#;

/// Initialize the knowledge graph schema in PostgreSQL
///
/// This function is idempotent - it can be safely called multiple times.
/// Uses IF NOT EXISTS clauses to avoid errors if tables already exist.
pub async fn initialize_schema(client: &Client) -> Result<()> {
    client
        .batch_execute(KG_SCHEMA)
        .await
        .map_err(|e| KgError::Schema(format!("Failed to initialize KG schema: {}", e)))?;

    Ok(())
}

/// Check if the knowledge graph schema has been initialized
///
/// Returns `true` if all required tables exist, `false` otherwise.
pub async fn is_schema_initialized(client: &Client) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT COUNT(*) as count FROM information_schema.tables
             WHERE table_schema = 'public' 
             AND table_name IN ('kg_repositories', 'kg_components', 'kg_entities', 'kg_relationships')",
            &[],
        )
        .await
        .map_err(|e| KgError::Schema(format!("Failed to check schema: {}", e)))?;

    let count: i64 = row.get(0);
    Ok(count == 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_constants() {
        // Verify schema SQL contains required table definitions
        assert!(KG_SCHEMA.contains("kg_repositories"));
        assert!(KG_SCHEMA.contains("kg_components"));
        assert!(KG_SCHEMA.contains("kg_entities"));
        assert!(KG_SCHEMA.contains("kg_relationships"));
    }
}
