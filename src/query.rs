//! Natural language query functionality

use crate::{database::Database, embeddings::EmbeddingGenerator, Config, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolResult {
    pub id: i32,
    pub name: String,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub path: String,
    pub language: Option<String>,
    pub repo_path: String,
    pub repo_name: String,
    pub similarity: f64,
}

pub struct QueryEngine {
    db: Arc<Database>,
    embedder: Arc<EmbeddingGenerator>,
}

impl QueryEngine {
    pub fn new(config: Arc<Config>, db: Arc<Database>) -> Result<Self> {
        let embedder = Arc::new(EmbeddingGenerator::new(config)?);

        Ok(QueryEngine { db, embedder })
    }

    pub async fn search_symbols(
        &self,
        query: &str,
        limit: i64,
        repository_path: Option<&str>,
    ) -> Result<Vec<SymbolResult>> {
        let query_embedding = self.embedder.encode_query(query)?;

        let rows = self
            .db
            .search_symbols(&query_embedding, limit, repository_path)
            .await?;

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
