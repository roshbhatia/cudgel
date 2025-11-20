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

/// Scheduled indexing task
///
/// Represents a repository that should be automatically re-indexed on a schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Database primary key
    pub id: i32,
    /// Foreign key to repository
    pub repo_id: i32,
    /// Interval in hours (1-8760)
    pub interval_hours: i32,
    /// Last time task was executed
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Next scheduled execution time
    pub next_run_at: chrono::DateTime<chrono::Utc>,
    /// Task status ("idle", "running", "failed", "paused", "cancelled")
    pub status: String,
    /// Version for optimistic locking
    pub version: i32,
    /// Number of retry attempts for failed tasks
    pub retry_count: i32,
    /// Error message if task failed
    pub error_message: Option<String>,
    /// When task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
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

    /// Get a database client from the pool
    ///
    /// This is a public accessor for the KG module to execute custom queries.
    pub async fn get_pool_client(&self) -> Result<deadpool_postgres::Object> {
        self.pool
            .get()
            .await
            .map_err(|e| crate::Error::from(e).with_context())
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
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_repo_id ON scheduled_tasks(repo_id)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_next_run ON scheduled_tasks(next_run_at) WHERE status = 'idle'",
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

    // Scheduled Tasks Operations

    /// Create a new scheduled task for a repository
    pub async fn create_scheduled_task(&self, repo_id: i32, interval_hours: i32) -> Result<i32> {
        let client = self.pool.get().await?;

        // Calculate next_run_at based on interval
        let next_run_at = chrono::Utc::now() + chrono::Duration::hours(interval_hours as i64);

        let row = client
            .query_one(
                "INSERT INTO scheduled_tasks (repo_id, interval_hours, next_run_at)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (repo_id)
                 DO UPDATE SET interval_hours = $2, next_run_at = $3
                 RETURNING id",
                &[&repo_id, &interval_hours, &next_run_at],
            )
            .await?;

        Ok(row.get("id"))
    }

    /// Delete a scheduled task for a repository
    pub async fn delete_scheduled_task(&self, repo_id: i32) -> Result<u64> {
        let client = self.pool.get().await?;

        let rows_affected = client
            .execute(
                "DELETE FROM scheduled_tasks WHERE repo_id = $1",
                &[&repo_id],
            )
            .await?;

        Ok(rows_affected)
    }

    /// Get all scheduled tasks
    pub async fn get_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT id, repo_id, interval_hours, last_run_at, next_run_at, status, version, retry_count, error_message, created_at
                 FROM scheduled_tasks
                 WHERE status IN ('idle', 'running')
                 ORDER BY next_run_at",
                &[],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|row| ScheduledTask {
                id: row.get("id"),
                repo_id: row.get("repo_id"),
                interval_hours: row.get("interval_hours"),
                last_run_at: row.get("last_run_at"),
                next_run_at: row.get("next_run_at"),
                status: row.get("status"),
                version: row.get("version"),
                retry_count: row.get("retry_count"),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    /// Get tasks that are due to run (next_run_at <= now)
    pub async fn get_due_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let client = self.pool.get().await?;

        let now = chrono::Utc::now();
        let rows = client
            .query(
                "SELECT id, repo_id, interval_hours, last_run_at, next_run_at, status, version, retry_count, error_message, created_at
                 FROM scheduled_tasks
                 WHERE next_run_at <= $1 AND status = 'idle'
                 ORDER BY next_run_at",
                &[&now],
            )
            .await?;

        Ok(rows
            .iter()
            .map(|row| ScheduledTask {
                id: row.get("id"),
                repo_id: row.get("repo_id"),
                interval_hours: row.get("interval_hours"),
                last_run_at: row.get("last_run_at"),
                next_run_at: row.get("next_run_at"),
                status: row.get("status"),
                version: row.get("version"),
                retry_count: row.get("retry_count"),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    /// Update task execution times after running
    pub async fn update_task_execution(
        &self,
        task_id: i32,
        last_run: chrono::DateTime<chrono::Utc>,
        next_run: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "UPDATE scheduled_tasks SET last_run_at = $1, next_run_at = $2 WHERE id = $3",
                &[&last_run, &next_run, &task_id],
            )
            .await?;

        Ok(())
    }

    /// Claim a task for execution using optimistic locking
    ///
    /// Atomically transitions task from 'idle' to 'running' status using version check.
    /// Returns Some(task) if claim succeeded, None if task was already claimed by another worker.
    pub async fn claim_task(
        &self,
        task_id: i32,
        expected_version: i32,
    ) -> Result<Option<ScheduledTask>> {
        let client = self.pool.get().await?;

        // Atomically update status to 'running' with version check
        let rows = client
            .query(
                "UPDATE scheduled_tasks 
                 SET status = 'running', version = version + 1
                 WHERE id = $1 AND version = $2 AND status = 'idle'
                 RETURNING id, repo_id, interval_hours, last_run_at, next_run_at, status, version, retry_count, error_message, created_at",
                &[&task_id, &expected_version],
            )
            .await?;

        if rows.is_empty() {
            // Task was already claimed by another worker or version mismatch
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(ScheduledTask {
            id: row.get("id"),
            repo_id: row.get("repo_id"),
            interval_hours: row.get("interval_hours"),
            last_run_at: row.get("last_run_at"),
            next_run_at: row.get("next_run_at"),
            status: row.get("status"),
            version: row.get("version"),
            retry_count: row.get("retry_count"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
        }))
    }

    /// Complete a task successfully, resetting error state and scheduling next run
    ///
    /// Calculates next run time based on interval and resets retry_count/error_message.
    pub async fn complete_task(&self, task_id: i32, interval_hours: i32) -> Result<()> {
        let client = self.pool.get().await?;

        let now = chrono::Utc::now();
        let next_run = now + chrono::Duration::hours(interval_hours as i64);

        client
            .execute(
                "UPDATE scheduled_tasks 
                 SET last_run_at = $1, 
                     next_run_at = $2, 
                     status = 'idle',
                     retry_count = 0,
                     error_message = NULL,
                     version = version + 1
                 WHERE id = $3",
                &[&now, &next_run, &task_id],
            )
            .await?;

        Ok(())
    }

    /// Mark a task as failed with retry logic
    ///
    /// Implements exponential backoff: retries after 1min, 2min, 4min, 8min, 16min.
    /// After 5 failures, sets status to 'failed' and stops retrying.
    pub async fn fail_task(&self, task_id: i32, error_msg: &str) -> Result<()> {
        let client = self.pool.get().await?;

        // Get current retry count
        let row = client
            .query_one(
                "SELECT retry_count, interval_hours FROM scheduled_tasks WHERE id = $1",
                &[&task_id],
            )
            .await?;

        let retry_count: i32 = row.get("retry_count");
        let _interval_hours: i32 = row.get("interval_hours");
        let new_retry_count = retry_count + 1;

        const MAX_RETRIES: i32 = 5;

        if new_retry_count >= MAX_RETRIES {
            // Max retries reached, mark as permanently failed
            client
                .execute(
                    "UPDATE scheduled_tasks 
                     SET status = 'failed',
                         retry_count = $1,
                         error_message = $2,
                         version = version + 1
                     WHERE id = $3",
                    &[&new_retry_count, &error_msg, &task_id],
                )
                .await?;
        } else {
            // Calculate exponential backoff: 2^retry_count minutes
            let backoff_minutes = 2_i64.pow(retry_count as u32);
            let now = chrono::Utc::now();
            let next_retry = now + chrono::Duration::minutes(backoff_minutes);

            client
                .execute(
                    "UPDATE scheduled_tasks 
                     SET status = 'idle',
                         retry_count = $1,
                         error_message = $2,
                         next_run_at = $3,
                         version = version + 1
                     WHERE id = $4",
                    &[&new_retry_count, &error_msg, &next_retry, &task_id],
                )
                .await?;
        }

        Ok(())
    }

    /// Get repository by ID
    pub async fn get_repository(&self, repository_id: i32) -> Result<Option<Repository>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT id, path, name, indexed_at, last_updated, metadata FROM repositories WHERE id = $1",
                &[&repository_id],
            )
            .await?;

        Ok(row.map(|r| Repository {
            id: r.get("id"),
            path: r.get("path"),
            name: r.get("name"),
            indexed_at: r.get("indexed_at"),
            last_updated: r.get("last_updated"),
            metadata: r.get("metadata"),
        }))
    }

    /// Initialize knowledge graph schema in PostgreSQL
    ///
    /// Creates additional tables for the knowledge graph feature.
    /// This function is idempotent - safe to call multiple times.
    pub async fn init_kg_schema(&self) -> Result<()> {
        let client = self.pool.get().await?;
        crate::kg::schema::initialize_schema(&*client).await
            .map_err(|e| crate::Error::Other(format!("KG schema init failed: {}", e)))?;
        Ok(())
    }

    /// Check if knowledge graph schema is initialized
    pub async fn is_kg_schema_initialized(&self) -> Result<bool> {
        let client = self.pool.get().await?;
        crate::kg::schema::is_schema_initialized(&*client).await
            .map_err(|e| crate::Error::Other(format!("KG schema check failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, EmbeddingConfig, IndexingConfig};

    #[test]
    fn test_database_url_formatting() {
        let config = Config {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 54321,
                database: "cudgel".to_string(),
                user: "testuser".to_string(),
                password: "testpass".to_string(),
            },
            embedding: EmbeddingConfig {
                model_path: std::path::PathBuf::from("/tmp/models"),
                dimension: 384,
                strategy: "onnx".to_string(),
            },
            indexing: IndexingConfig {
                batch_size: 100,
                max_file_size: 1024 * 1024,
            },
        };

        let url = config.database_url();
        assert_eq!(
            url,
            "host=localhost port=54321 dbname=cudgel user=testuser password=testpass"
        );
    }

    #[test]
    fn test_database_url_special_characters() {
        let config = Config {
            database: DatabaseConfig {
                host: "db.example.com".to_string(),
                port: 5432,
                database: "my-database".to_string(),
                user: "user@domain".to_string(),
                password: "p@ssw0rd!".to_string(),
            },
            embedding: EmbeddingConfig {
                model_path: std::path::PathBuf::from("/tmp/models"),
                dimension: 384,
                strategy: "onnx".to_string(),
            },
            indexing: IndexingConfig {
                batch_size: 100,
                max_file_size: 1024 * 1024,
            },
        };

        let url = config.database_url();
        assert_eq!(
            url,
            "host=db.example.com port=5432 dbname=my-database user=user@domain password=p@ssw0rd!"
        );
    }

    #[test]
    fn test_scheduled_task_struct_creation() {
        let now = chrono::Utc::now();
        let task = ScheduledTask {
            id: 1,
            repo_id: 10,
            interval_hours: 24,
            last_run_at: Some(now),
            next_run_at: now + chrono::Duration::hours(24),
            status: "idle".to_string(),
            version: 1,
            retry_count: 0,
            error_message: None,
            created_at: now,
        };

        assert_eq!(task.id, 1);
        assert_eq!(task.repo_id, 10);
        assert_eq!(task.interval_hours, 24);
        assert_eq!(task.status, "idle");
        assert_eq!(task.version, 1);
        assert_eq!(task.retry_count, 0);
        assert!(task.last_run_at.is_some());
        assert!(task.error_message.is_none());
    }

    #[test]
    fn test_repository_struct_creation() {
        let now = chrono::Utc::now().naive_utc();
        let repo = Repository {
            id: 1,
            path: "/test/path".to_string(),
            name: "test_repo".to_string(),
            indexed_at: now,
            last_updated: now,
            metadata: serde_json::json!({}),
        };

        assert_eq!(repo.id, 1);
        assert_eq!(repo.path, "/test/path");
        assert_eq!(repo.name, "test_repo");
        assert_eq!(repo.indexed_at, now);
        assert_eq!(repo.last_updated, now);
    }
}
