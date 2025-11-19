//! Natural language query functionality

use crate::{database::Database, embeddings::EmbedderBackend, Config, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Symbol search result
///
/// Contains symbol metadata and similarity score from vector search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolResult {
    /// Symbol database ID
    pub id: i32,
    /// Symbol name
    pub name: String,
    /// Symbol kind ("function", "class", etc.)
    pub kind: String,
    /// Full signature (for functions/methods)
    pub signature: Option<String>,
    /// Documentation string
    pub docstring: Option<String>,
    /// Starting line number (1-indexed)
    pub start_line: i32,
    /// Ending line number (1-indexed)
    pub end_line: i32,
    /// File path
    pub path: String,
    /// Programming language
    pub language: Option<String>,
    /// Repository path
    pub repo_path: String,
    /// Repository name
    pub repo_name: String,
    /// Similarity score (0.0 to 1.0, higher is more similar)
    pub similarity: f64,
}

/// Natural language query engine
///
/// Performs semantic search over indexed code using embeddings.
pub struct QueryEngine {
    db: Arc<Database>,
    embedder: Arc<EmbedderBackend>,
}

impl QueryEngine {
    /// Create a new query engine
    ///
    /// # Arguments
    /// * `config` - Application configuration
    /// * `db` - Database connection
    ///
    /// # Returns
    /// Query engine ready to process searches
    pub fn new(config: Arc<Config>, db: Arc<Database>) -> Result<Self> {
        let embedder = Arc::new(EmbedderBackend::from_config(&config)?);

        Ok(QueryEngine { db, embedder })
    }

    /// Search for symbols using natural language query
    ///
    /// Converts the query to an embedding and performs vector similarity search.
    ///
    /// # Arguments
    /// * `query` - Natural language search query
    /// * `limit` - Maximum number of results
    /// * `repository_path` - Optional filter to specific repository
    ///
    /// # Returns
    /// Vector of symbol results ordered by similarity (most similar first)
    pub async fn search_symbols(
        &self,
        query: &str,
        limit: i64,
        repository_path: Option<&str>,
    ) -> Result<Vec<SymbolResult>> {
        let query_embedding = self.embedder.encode(query)?;

        // Debug: Check if embedding is valid
        if std::env::var("CUDGEL_DEBUG").is_ok() {
            eprintln!("Query: \"{}\"", query);
            eprintln!("Embedding length: {}", query_embedding.len());
            eprintln!(
                "First 5 values: {:?}",
                &query_embedding[..5.min(query_embedding.len())]
            );
            let norm: f32 = query_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            eprintln!("Embedding L2 norm: {:.6}", norm);
        }

        let rows = self
            .db
            .search_symbols(&query_embedding, limit, repository_path)
            .await?;

        if std::env::var("CUDGEL_DEBUG").is_ok() {
            eprintln!("Query returned {} rows", rows.len());
            for (i, row) in rows.iter().enumerate().take(3) {
                let name: String = row.get("name");
                let similarity: f64 = row.get("similarity");
                eprintln!(
                    "  Result {}: {} (similarity: {:.6})",
                    i + 1,
                    name,
                    similarity
                );
            }
        }

        let results = rows
            .iter()
            .map(|row| SymbolResult {
                id: row.get("id"),
                name: row.get("name"),
                kind: row.get("kind"),
                signature: row.get("signature"),
                docstring: row.get("docstring"),
                start_line: row.get("start_line"),
                end_line: row.get("end_line"),
                path: row.get("path"),
                language: row.get("language"),
                repo_path: row.get("repo_path"),
                repo_name: row.get("repo_name"),
                similarity: row.get("similarity"),
            })
            .collect();

        Ok(results)
    }
}
