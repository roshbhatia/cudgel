// src/deps/schema.rs
//! Database schema initialization and validation

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use tokio_postgres::{Client, NoTls};

/// Schema initialization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaState {
    NotInitialized,
    Initializing,
    Initialized,
    Error,
}

/// Represents database schema version and state
#[derive(Debug, Clone)]
pub struct SchemaVersion {
    pub version_number: i32,
    pub applied_at: Option<DateTime<Utc>>,
    pub tables_created: Vec<String>,
    pub indexes_created: Vec<String>,
    pub extensions_enabled: Vec<String>,
    pub is_initialized: bool,
}

impl SchemaVersion {
    /// Create a new schema version
    pub fn new(version_number: i32) -> Self {
        Self {
            version_number,
            applied_at: None,
            tables_created: Vec::new(),
            indexes_created: Vec::new(),
            extensions_enabled: Vec::new(),
            is_initialized: false,
        }
    }
}

/// Schema initializer for database setup
pub struct SchemaInitializer {
    host: String,
    port: u16,
    database: String,
    user: String,
}

impl SchemaInitializer {
    /// Create a new schema initializer
    pub fn new(host: String, port: u16, database: String, user: String) -> Self {
        Self {
            host,
            port,
            database,
            user,
        }
    }

    /// Get a database client connection
    async fn get_client(&self) -> Result<Client> {
        let conn_str = format!(
            "host={} port={} dbname={} user={} password=",
            self.host, self.port, self.database, self.user
        );

        let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
            .await
            .map_err(|e| Error::DatabaseConnectionFailed(e.to_string()))?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(client)
    }

    /// Check if schema is initialized
    pub async fn check_initialized(&self) -> Result<bool> {
        let client = self.get_client().await?;

        let row = client
            .query_opt(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables
                    WHERE table_name = 'repositories'
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(e.to_string()))?;

        if let Some(row) = row {
            let exists: bool = row.get(0);
            Ok(exists)
        } else {
            Ok(false)
        }
    }

    /// Initialize database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        let client = self.get_client().await?;

        // Enable pgvector extension
        client
            .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create vector extension: {}", e)))?;

        // Repositories table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS repositories (
                    id SERIAL PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    metadata JSONB DEFAULT '{}'
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create repositories table: {}", e)))?;

        // Files table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS files (
                    id SERIAL PRIMARY KEY,
                    repository_id INTEGER REFERENCES repositories(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    language TEXT,
                    content TEXT,
                    hash TEXT NOT NULL,
                    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    metadata JSONB DEFAULT '{}',
                    UNIQUE(repository_id, path)
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create files table: {}", e)))?;

        // AST nodes table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS ast_nodes (
                    id SERIAL PRIMARY KEY,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    parent_id INTEGER REFERENCES ast_nodes(id) ON DELETE CASCADE,
                    node_type TEXT NOT NULL,
                    text TEXT,
                    start_line INTEGER NOT NULL,
                    start_column INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    end_column INTEGER NOT NULL,
                    metadata JSONB DEFAULT '{}'
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create ast_nodes table: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ast_nodes_parent ON ast_nodes(parent_id)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create ast_nodes parent index: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ast_nodes_file ON ast_nodes(file_id)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create ast_nodes file index: {}", e)))?;

        // Symbols table
        let dimension = 384; // MiniLM embedding dimension
        client
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS symbols (
                        id SERIAL PRIMARY KEY,
                        file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                        ast_node_id INTEGER REFERENCES ast_nodes(id) ON DELETE CASCADE,
                        name TEXT NOT NULL,
                        kind TEXT NOT NULL,
                        signature TEXT,
                        docstring TEXT,
                        start_line INTEGER NOT NULL,
                        end_line INTEGER NOT NULL,
                        embedding vector({}),
                        metadata JSONB DEFAULT '{{}}'
                    )",
                    dimension
                ),
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create symbols table: {}", e)))?;

        // Vector index for symbols
        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_symbols_embedding
                 ON symbols USING hnsw (embedding vector_cosine_ops)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create symbols embedding index: {}", e)))?;

        // References table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS \"references\" (
                    id SERIAL PRIMARY KEY,
                    from_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    to_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    reference_type TEXT NOT NULL,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    line INTEGER NOT NULL,
                    \"column\" INTEGER NOT NULL,
                    metadata JSONB DEFAULT '{}',
                    UNIQUE(from_symbol_id, to_symbol_id, reference_type, line, \"column\")
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create references table: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_from ON \"references\"(from_symbol_id)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create references from index: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_to ON \"references\"(to_symbol_id)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create references to index: {}", e)))?;

        // Code chunks table
        client
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS code_chunks (
                        id SERIAL PRIMARY KEY,
                        file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                        symbol_id INTEGER REFERENCES symbols(id) ON DELETE SET NULL,
                        text TEXT NOT NULL,
                        start_line INTEGER NOT NULL,
                        end_line INTEGER NOT NULL,
                        embedding vector({}),
                        metadata JSONB DEFAULT '{{}}'
                    )",
                    dimension
                ),
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create code_chunks table: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_code_chunks_embedding
                 ON code_chunks USING hnsw (embedding vector_cosine_ops)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create code_chunks embedding index: {}", e)))?;

        // Scheduled tasks table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS scheduled_tasks (
                    id SERIAL PRIMARY KEY,
                    repo_id INTEGER NOT NULL UNIQUE REFERENCES repositories(id) ON DELETE CASCADE,
                    interval_hours INTEGER NOT NULL CHECK (interval_hours > 0 AND interval_hours <= 8760),
                    next_run_at TIMESTAMPTZ NOT NULL,
                    last_run_at TIMESTAMPTZ,
                    status TEXT DEFAULT 'idle' CHECK (status IN ('idle', 'running', 'failed', 'paused', 'cancelled')),
                    version INTEGER DEFAULT 1 CHECK (version > 0),
                    retry_count INTEGER DEFAULT 0 CHECK (retry_count >= 0),
                    error_message TEXT,
                    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
                )",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create scheduled_tasks table: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_repo_id ON scheduled_tasks(repo_id)",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create scheduled_tasks repo_id index: {}", e)))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_next_run ON scheduled_tasks(next_run_at) WHERE status = 'idle'",
                &[],
            )
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to create scheduled_tasks next_run index: {}", e)))?;

        Ok(())
    }

    /// Verify required extensions are installed
    pub async fn verify_extensions(&self) -> Result<()> {
        let client = self.get_client().await?;

        let row = client
            .query_opt("SELECT 1 FROM pg_extension WHERE extname = 'vector'", &[])
            .await
            .map_err(|e| Error::SchemaInitFailed(format!("Failed to check pgvector: {}", e)))?;

        if row.is_none() {
            return Err(Error::MissingDependency(
                "pgvector extension is not installed".to_string(),
            ));
        }

        Ok(())
    }
}
