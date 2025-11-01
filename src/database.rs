//! Database layer with PostgreSQL and pgvector

use crate::{Config, Result};
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio_postgres::{NoTls, Row};

/// Database connection pool manager
///
/// Provides async access to PostgreSQL with pgvector extension for vector similarity search.
/// Uses connection pooling via deadpool-postgres for efficient resource management.
#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool,
}

/// Indexed code repository record
///
/// Represents a repository that has been indexed by cudgel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Database primary key
    pub id: i32,
    /// Absolute file system path to the repository
    pub path: String,
    /// Repository name (derived from path)
    pub name: String,
    /// Timestamp when first indexed
    pub indexed_at: chrono::NaiveDateTime,
    /// Timestamp of last update/re-index
    pub last_updated: chrono::NaiveDateTime,
    /// Additional repository metadata (JSON)
    pub metadata: JsonValue,
}

/// Source code file record
///
/// Represents a single source file that has been indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Database primary key
    pub id: i32,
    /// Foreign key to parent repository
    pub repository_id: i32,
    /// Relative path within repository
    pub path: String,
    /// Detected programming language (e.g., "rust", "python")
    pub language: Option<String>,
    /// Full file contents
    pub content: String,
    /// SHA256 hash of content for change detection
    pub hash: String,
    /// Timestamp when indexed
    pub indexed_at: chrono::NaiveDateTime,
    /// Additional file metadata (JSON)
    pub metadata: JsonValue,
}

/// Code symbol (function, class, method, etc.)
///
/// Represents an extracted symbol with its location and documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Database primary key
    pub id: i32,
    /// Foreign key to source file
    pub file_id: i32,
    /// Symbol name (function name, class name, etc.)
    pub name: String,
    /// Symbol kind ("function", "class", "method", "struct", etc.)
    pub kind: String,
    /// Full signature (for functions/methods)
    pub signature: Option<String>,
    /// Documentation string
    pub docstring: Option<String>,
    /// Starting line number (1-indexed)
    pub start_line: i32,
    /// Ending line number (1-indexed)
    pub end_line: i32,
}

/// Symbol reference/relationship record
///
/// Represents a relationship between two symbols (calls, imports, extends, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Database primary key
    pub id: i32,
    /// Source symbol ID
    pub from_symbol_id: i32,
    /// Target symbol ID
    pub to_symbol_id: i32,
    /// Type of reference ("call", "import", "extends", etc.)
    pub reference_type: String,
    /// File where reference occurs
    pub file_id: i32,
    /// Line number of reference (1-indexed)
    pub line: i32,
    /// Column number of reference (0-indexed)
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
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| crate::Error::from(e).with_context())?;
        let exists = client
            .query_opt(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables
                    WHERE table_name = 'repositories'
                )",
                &[],
            )
            .await
            .map_err(|e| crate::Error::from(e).with_context())?;

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
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| crate::Error::from(e).with_context())?;
        let row = client
            .query_one("SELECT 1 as health", &[])
            .await
            .map_err(|e| crate::Error::from(e).with_context())?;
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

    /// Reset database schema
    ///
    /// Drops all tables and recreates them. WARNING: This deletes all data!
    ///
    /// # Returns
    /// Ok if schema is successfully reset
    pub async fn reset_schema(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Drop tables in reverse dependency order
        // Note: "references" is a reserved keyword, so it needs to be quoted
        client
            .execute("DROP TABLE IF EXISTS knowledge_documents CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS scheduled_tasks CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS code_chunks CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS \"references\" CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS symbols CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS ast_nodes CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS files CASCADE", &[])
            .await?;
        client
            .execute("DROP TABLE IF EXISTS repositories CASCADE", &[])
            .await?;

        // Recreate schema
        self.init_schema().await
    }

    /// Initialize database schema
    ///
    /// Creates all tables, indexes, and extensions required for cudgel.
    /// Safe to call multiple times (uses IF NOT EXISTS).
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
        // Use HNSW instead of IVFFlat for better recall on small-medium datasets
        // HNSW provides exact nearest neighbor search with good performance
        // IVFFlat is better for very large datasets (millions of vectors)
        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_symbols_embedding
                 ON symbols USING hnsw (embedding vector_cosine_ops)",
                &[],
            )
            .await?;

        // References table (quoted because "references" is a reserved keyword)
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
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_from ON \"references\"(from_symbol_id)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_references_to ON \"references\"(to_symbol_id)",
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
                 ON code_chunks USING hnsw (embedding vector_cosine_ops)",
                &[],
            )
            .await?;

        // Scheduled tasks table (User Story 2: Automatic Re-indexing)
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS scheduled_tasks (
                    id SERIAL PRIMARY KEY,
                    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                    interval_hours INTEGER NOT NULL CHECK (interval_hours > 0 AND interval_hours <= 8760),
                    next_run_at TIMESTAMPTZ NOT NULL,
                    last_run_at TIMESTAMPTZ,
                    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'paused', 'cancelled')),
                    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
                )",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_repo_id ON scheduled_tasks(repo_id)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_next_run ON scheduled_tasks(next_run_at) WHERE status = 'active'",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_status ON scheduled_tasks(status)",
                &[],
            )
            .await?;

        // Knowledge documents table (User Story 3: Knowledge Graph Generation)
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS knowledge_documents (
                    id SERIAL PRIMARY KEY,
                    repo_id INTEGER NOT NULL UNIQUE REFERENCES repositories(id) ON DELETE CASCADE,
                    content TEXT NOT NULL CHECK (content != ''),
                    generated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                    last_edited_at TIMESTAMPTZ,
                    version INTEGER DEFAULT 1 CHECK (version > 0)
                )",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_knowledge_documents_repo_id ON knowledge_documents(repo_id)",
                &[],
            )
            .await?;

        Ok(())
    }

    /// Add or update a repository
    ///
    /// Creates a new repository record or updates the last_updated timestamp if it already exists.
    ///
    /// # Arguments
    /// * `path` - Absolute file system path to repository
    /// * `name` - Repository name
    ///
    /// # Returns
    /// Repository ID (either newly created or existing)
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

    /// Add or update a file
    ///
    /// Creates a new file record or updates content/hash if the file already exists.
    ///
    /// # Arguments
    /// * `repository_id` - Parent repository ID
    /// * `path` - Relative path within repository
    /// * `language` - Detected programming language
    /// * `content` - Full file contents
    /// * `hash` - SHA256 hash of content
    ///
    /// # Returns
    /// File ID (either newly created or existing)
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

    /// Add a symbol with its embedding
    ///
    /// Inserts a new symbol record with semantic embedding for vector search.
    ///
    /// # Arguments
    /// * `file_id` - Parent file ID
    /// * `name` - Symbol name
    /// * `kind` - Symbol kind ("function", "class", etc.)
    /// * `signature` - Full signature (for functions/methods)
    /// * `docstring` - Documentation string
    /// * `start_line` - Starting line number (1-indexed)
    /// * `end_line` - Ending line number (1-indexed)
    /// * `embedding` - Vector embedding (384 dimensions)
    ///
    /// # Returns
    /// Symbol ID
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

    /// Search for symbols using vector similarity
    ///
    /// Performs approximate nearest neighbor search using pgvector's IVFFlat index.
    /// Results are ordered by cosine similarity (most similar first).
    ///
    /// # Arguments
    /// * `query_embedding` - Query vector (384 dimensions)
    /// * `limit` - Maximum number of results
    /// * `repository_path` - Optional filter to specific repository
    ///
    /// # Returns
    /// Vector of database rows containing symbol data and similarity scores
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

    /// Get symbol by exact name match
    ///
    /// # Arguments
    /// * `name` - Exact symbol name to search for
    /// * `repository_path` - Optional filter to specific repository
    ///
    /// # Returns
    /// Symbol row if found, None otherwise
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

    /// Get symbol by database ID
    ///
    /// # Arguments
    /// * `symbol_id` - Symbol primary key
    ///
    /// # Returns
    /// Symbol row if found, None otherwise
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

    /// Get all outgoing references from a symbol
    ///
    /// Returns symbols that this symbol references (calls, imports, etc.).
    ///
    /// # Arguments
    /// * `symbol_id` - Source symbol ID
    ///
    /// # Returns
    /// Vector of reference records
    pub async fn get_references(&self, symbol_id: i32) -> Result<Vec<Reference>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT r.*, s.name as to_name, s.kind as to_kind
                 FROM \"references\" r
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

    pub async fn get_file_hash(&self, repository_id: i32, path: &str) -> Result<Option<String>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT hash FROM files WHERE repository_id = $1 AND path = $2",
                &[&repository_id, &path],
            )
            .await?;

        Ok(row.map(|r| r.get("hash")))
    }

    pub async fn delete_file_symbols(&self, file_id: i32) -> Result<u64> {
        let client = self.pool.get().await?;

        let rows_affected = client
            .execute("DELETE FROM symbols WHERE file_id = $1", &[&file_id])
            .await?;

        Ok(rows_affected)
    }

    pub async fn get_file_id(&self, repository_id: i32, path: &str) -> Result<Option<i32>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT id FROM files WHERE repository_id = $1 AND path = $2",
                &[&repository_id, &path],
            )
            .await?;

        Ok(row.map(|r| r.get("id")))
    }

    pub async fn delete_repository_symbols(&self, repository_id: i32) -> Result<u64> {
        let client = self.pool.get().await?;

        let rows_affected = client
            .execute(
                "DELETE FROM symbols WHERE file_id IN (SELECT id FROM files WHERE repository_id = $1)",
                &[&repository_id],
            )
            .await?;

        Ok(rows_affected)
    }
}
