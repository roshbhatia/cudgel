//! Cudgel - A code indexing tool with tree-sitter, Temporal, and PostgreSQL/pgvector

pub mod config;
pub mod database;
pub mod embeddings;
pub mod error;
pub mod graph;
pub mod indexer;
pub mod local_db;
pub mod lsp;
pub mod macos_services;
pub mod parser;
pub mod query;
pub mod services;
pub mod temporal;

pub use config::Config;
pub use error::{Error, Result};
pub use indexer::IndexingStats;
