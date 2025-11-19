//! Cudgel - A code indexing tool with tree-sitter and PostgreSQL/pgvector

pub mod config;
pub mod database;
pub mod deps;
pub mod embeddings;
pub mod error;
pub mod graph;
pub mod indexer;
pub mod kg;
pub mod llm;
pub mod orchestrator;
pub mod parser;
pub mod query;

pub use config::Config;
pub use error::{Error, Result};
pub use indexer::IndexingStats;
