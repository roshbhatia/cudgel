//! Database layer with PostgreSQL and pgvector

use crate::{Config, Result};
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio_postgres::{NoTls, Row};

#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i32,
    pub path: String,
    pub name: String,
    pub indexed_at: chrono::NaiveDateTime,
    pub last_updated: chrono::NaiveDateTime,
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i32,
    pub repository_id: i32,
    pub path: String,
    pub language: Option<String>,
    pub content: String,
    pub hash: String,
    pub indexed_at: chrono::NaiveDateTime,
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: i32,
    pub file_id: i32,
    pub name: String,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub id: i32,
    pub from_symbol_id: i32,
    pub to_symbol_id: i32,
    pub reference_type: String,
    pub file_id: i32,
    pub line: i32,
    pub column: i32,
}

impl Database {
    /// Create a new database connection and auto-initialize schema if needed
    pub async fn new(config: &Config) -> Result<Self> {
        let mut pg_config = PoolConfig::new();
        pg_config.host = Some(config.database.host.clone());
        pg_config.port = Some(config.database.port);
        pg_config.dbname = Some(config.database.database.clone());
        pg_config.user = Some(config.database.user.clone());
        pg_config.password = Some(config.database.password.clone());

        let pool = pg_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| crate::Error::PoolCreation(e.to_string()))?;

        let db = Database { pool };

        // Auto-initialize schema if not exists
        if let Err(e) = db.ensure_initialized().await {
            eprintln!("Warning: Could not auto-initialize database: {}", e);
        }

        Ok(db)
    }

    /// Ensure database schema is initialized
    async fn ensure_initialized(&self) -> Result<()> {
        // Check if schema exists by trying to query repositories table
        let client = self.pool.get().await?;
        let exists = client
            .query_opt(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables
                    WHERE table_name = 'repositories'
                )",
                &[],
            )
            .await?;

        if let Some(row) = exists {
            let schema_exists: bool = row.get(0);
            if !schema_exists {
                drop(client); // Release connection before init
                println!("Initializing database schema...");
                self.init_schema().await?;
                println!("Database schema initialized!");
            }
        }

        Ok(())
    }

    /// Check database connection health
    pub async fn health_check(&self) -> Result<bool> {
        let client = self.pool.get().await?;
        let row = client.query_one("SELECT 1 as health", &[]).await?;
        let health: i32 = row.get("health");
        Ok(health == 1)
    }

    /// Check if pgvector extension is installed
    pub async fn check_pgvector(&self) -> Result<bool> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt("SELECT 1 FROM pg_extension WHERE extname = 'vector'", &[])
            .await?;
        Ok(row.is_some())
    }

    pub async fn init_schema(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Enable pgvector extension
        client
            .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
            .await?;

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
            .await?;

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
            .await?;

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
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ast_nodes_parent ON ast_nodes(parent_id)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ast_nodes_file ON ast_nodes(file_id)",
                &[],
            )
            .await?;

        // Symbols table
        let dimension = 384; // TODO: Get from config
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
            .await?;

        // Vector index for symbols
        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_symbols_embedding
                 ON symbols USING ivfflat (embedding vector_cosine_ops)
                 WITH (lists = 100)",
                &[],
            )
            .await?;

        // References table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS references (
                    id SERIAL PRIMARY KEY,
                    from_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    to_symbol_id INTEGER REFERENCES symbols(id) ON DELETE CASCADE,
                    reference_type TEXT NOT NULL,
                    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                    line INTEGER NOT NULL,
                    column INTEGER NOT NULL,
                    metadata JSONB DEFAULT '{}',
                    UNIQUE(from_symbol_id, to_symbol_id, reference_type, line, column)
                )",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_from ON references(from_symbol_id)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_to ON references(to_symbol_id)",
                &[],
            )
            .await?;

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
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_code_chunks_embedding
                 ON code_chunks USING ivfflat (embedding vector_cosine_ops)
                 WITH (lists = 100)",
                &[],
            )
            .await?;

        Ok(())
    }

    pub async fn add_repository(&self, path: &str, name: &str) -> Result<i32> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "INSERT INTO repositories (path, name, metadata)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (path) DO UPDATE
                 SET last_updated = CURRENT_TIMESTAMP
                 RETURNING id",
                &[&path, &name, &serde_json::json!({})],
            )
            .await?;

        Ok(row.get(0))
    }

    pub async fn add_file(
        &self,
        repository_id: i32,
        path: &str,
        language: Option<&str>,
        content: &str,
        hash: &str,
    ) -> Result<i32> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "INSERT INTO files (repository_id, path, language, content, hash, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (repository_id, path) DO UPDATE
                 SET content = EXCLUDED.content,
                     hash = EXCLUDED.hash,
                     indexed_at = CURRENT_TIMESTAMP
                 RETURNING id",
                &[
                    &repository_id,
                    &path,
                    &language,
                    &content,
                    &hash,
                    &serde_json::json!({}),
                ],
            )
            .await?;

        Ok(row.get(0))
    }

    #[allow(clippy::too_many_arguments)] // All parameters are necessary for symbol data
    pub async fn add_symbol(
        &self,
        file_id: i32,
        name: &str,
        kind: &str,
        signature: Option<&str>,
        docstring: Option<&str>,
        start_line: i32,
        end_line: i32,
        embedding: &[f32],
    ) -> Result<i32> {
        let client = self.pool.get().await?;

        let vector = Vector::from(embedding.to_vec());

        let row = client
            .query_one(
                "INSERT INTO symbols (file_id, name, kind, signature, docstring, start_line, end_line, embedding)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 RETURNING id",
                &[
                    &file_id,
                    &name,
                    &kind,
                    &signature,
                    &docstring,
                    &start_line,
                    &end_line,
                    &vector,
                ],
            )
            .await?;

        Ok(row.get(0))
    }

    pub async fn search_symbols(
        &self,
        query_embedding: &[f32],
        limit: i64,
        repository_path: Option<&str>,
    ) -> Result<Vec<Row>> {
        let client = self.pool.get().await?;
        let vector = Vector::from(query_embedding.to_vec());

        let rows = if let Some(repo_path) = repository_path {
            client
                .query(
                    "SELECT
                        s.id, s.name, s.kind, s.signature, s.docstring,
                        s.start_line, s.end_line,
                        f.path, f.language,
                        r.path as repo_path, r.name as repo_name,
                        1 - (s.embedding <=> $1::vector) as similarity
                     FROM symbols s
                     JOIN files f ON s.file_id = f.id
                     JOIN repositories r ON f.repository_id = r.id
                     WHERE r.path = $2
                     ORDER BY s.embedding <=> $1::vector
                     LIMIT $3",
                    &[&vector, &repo_path, &limit],
                )
                .await?
        } else {
            client
                .query(
                    "SELECT
                        s.id, s.name, s.kind, s.signature, s.docstring,
                        s.start_line, s.end_line,
                        f.path, f.language,
                        r.path as repo_path, r.name as repo_name,
                        1 - (s.embedding <=> $1::vector) as similarity
                     FROM symbols s
                     JOIN files f ON s.file_id = f.id
                     JOIN repositories r ON f.repository_id = r.id
                     ORDER BY s.embedding <=> $1::vector
                     LIMIT $2",
                    &[&vector, &limit],
                )
                .await?
        };

        Ok(rows)
    }

    pub async fn get_symbol_by_name(
        &self,
        name: &str,
        repository_path: Option<&str>,
    ) -> Result<Option<Row>> {
        let client = self.pool.get().await?;

        let row = if let Some(repo_path) = repository_path {
            client
                .query_opt(
                    "SELECT s.*, f.path, f.language, r.path as repo_path
                     FROM symbols s
                     JOIN files f ON s.file_id = f.id
                     JOIN repositories r ON f.repository_id = r.id
                     WHERE s.name = $1 AND r.path = $2
                     LIMIT 1",
                    &[&name, &repo_path],
                )
                .await?
        } else {
            client
                .query_opt(
                    "SELECT s.*, f.path, f.language, r.path as repo_path
                     FROM symbols s
                     JOIN files f ON s.file_id = f.id
                     JOIN repositories r ON f.repository_id = r.id
                     WHERE s.name = $1
                     LIMIT 1",
                    &[&name],
                )
                .await?
        };

        Ok(row)
    }

    pub async fn get_symbol_by_id(&self, symbol_id: i32) -> Result<Option<Row>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT s.*, f.path, f.language, r.path as repo_path
                 FROM symbols s
                 JOIN files f ON s.file_id = f.id
                 JOIN repositories r ON f.repository_id = r.id
                 WHERE s.id = $1",
                &[&symbol_id],
            )
            .await?;

        Ok(row)
    }

    pub async fn get_references(&self, symbol_id: i32) -> Result<Vec<Reference>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT r.*, s.name as to_name, s.kind as to_kind
                 FROM references r
                 JOIN symbols s ON r.to_symbol_id = s.id
                 WHERE r.from_symbol_id = $1",
                &[&symbol_id],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|row| Reference {
                id: row.get("id"),
                from_symbol_id: row.get("from_symbol_id"),
                to_symbol_id: row.get("to_symbol_id"),
                reference_type: row.get("reference_type"),
                file_id: row.get("file_id"),
                line: row.get("line"),
                column: row.get("column"),
            })
            .collect())
    }
}
