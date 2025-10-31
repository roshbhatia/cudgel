//! Code indexing functionality
//!
//! This module provides the core indexing engine for Cudgel. It walks directory trees,
//! parses source files using tree-sitter, extracts symbols, generates embeddings, and
//! stores everything in PostgreSQL with pgvector.
//!
//! # Example
//!
//! ```no_run
//! use cudgel::{Config, database::Database, indexer::Indexer};
//! use std::sync::Arc;
//! use std::path::Path;
//!
//! # async fn example() -> cudgel::Result<()> {
//! let config = Arc::new(Config::from_env()?);
//! let db = Arc::new(Database::new(&config).await?);
//! let mut indexer = Indexer::new(config, db)?;
//!
//! let (repo_id, stats) = indexer.index_repository(Path::new("/path/to/repo")).await?;
//! println!("Indexed {} files with {} symbols", stats.indexed_files, stats.total_symbols);
//! # Ok(())
//! # }
//! ```

use crate::{
    database::Database,
    embeddings::EmbeddingGenerator,
    parser::{CodeParser, Symbol},
    Config, Result,
};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

/// Statistics collected during repository indexing.
///
/// Provides comprehensive metrics about the indexing process including success/failure
/// counts, language distribution, symbol kinds, and error messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStats {
    /// Total number of files discovered in the repository
    pub total_files: usize,
    /// Number of files successfully indexed
    pub indexed_files: usize,
    /// Number of files that failed to index
    pub failed_files: usize,
    /// Total number of symbols (functions, classes, etc.) extracted
    pub total_symbols: usize,
    /// Count of symbols by their kind (function, class, method, etc.)
    pub symbols_by_kind: HashMap<String, usize>,
    /// Count of files by programming language
    pub files_by_language: HashMap<String, usize>,
    /// Error messages from failed indexing operations (limited to first 10)
    pub errors: Vec<String>,
}

/// The main indexing engine for Cudgel.
///
/// Handles walking directory trees, parsing source files, extracting symbols,
/// and storing indexed data in PostgreSQL.
pub struct Indexer {
    config: Arc<Config>,
    db: Arc<Database>,
    parser: CodeParser,
    embedder: Arc<EmbeddingGenerator>,
}

impl Indexer {
    /// Create a new indexer instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `db` - Database connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if the embedding generator cannot be initialized.
    pub fn new(config: Arc<Config>, db: Arc<Database>) -> Result<Self> {
        let embedder = Arc::new(EmbeddingGenerator::new(config.clone())?);

        Ok(Indexer {
            config,
            db,
            parser: CodeParser::new(),
            embedder,
        })
    }

    /// Index an entire repository.
    ///
    /// Walks the directory tree, identifies source files, parses them, extracts symbols,
    /// generates embeddings, and stores everything in the database. Displays a progress
    /// bar during indexing.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root directory
    ///
    /// # Returns
    ///
    /// Returns a tuple of (repository_id, indexing_statistics) on success.
    ///
    /// # Errors
    ///
    /// May return errors for:
    /// - Invalid repository path
    /// - Database connection failures
    /// - File read errors
    /// - Parsing errors
    ///
    /// Individual file failures are captured in the statistics and don't stop indexing.
    pub async fn index_repository(&mut self, repo_path: &Path) -> Result<(i32, IndexingStats)> {
        // Validate input path
        if !repo_path.exists() {
            return Err(crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Repository path does not exist: {}", repo_path.display()),
            )));
        }

        if !repo_path.is_dir() {
            return Err(crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Repository path is not a directory: {}",
                    repo_path.display()
                ),
            )));
        }

        let name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let repo_path_str = repo_path.to_string_lossy().to_string();

        // Add repository to database
        let repo_id = self.db.add_repository(&repo_path_str, name).await?;

        // Find all source files
        let files = self.find_source_files(repo_path)?;
        let total_files = files.len();

        println!("Found {} files to index", total_files);

        let pb = ProgressBar::new(total_files as u64);
        let style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-");
        pb.set_style(style);

        let mut stats = IndexingStats {
            total_files,
            indexed_files: 0,
            failed_files: 0,
            total_symbols: 0,
            symbols_by_kind: HashMap::new(),
            files_by_language: HashMap::new(),
            errors: Vec::new(),
        };

        for file_path in files {
            match self.index_file(repo_id, &file_path, &mut stats).await {
                Ok(_) => {
                    stats.indexed_files += 1;
                    pb.set_message(format!("Indexing: {}", file_path.display()));
                }
                Err(e) => {
                    stats.failed_files += 1;
                    let error_msg = format!("Error indexing {:?}: {}", file_path, e);
                    eprintln!("{}", error_msg);
                    if stats.errors.len() < 10 {
                        stats.errors.push(error_msg);
                    }
                }
            }
            pb.inc(1);
        }

        pb.finish_with_message(format!(
            "Indexed {} files ({} succeeded, {} failed, {} symbols)",
            total_files, stats.indexed_files, stats.failed_files, stats.total_symbols
        ));

        Ok((repo_id, stats))
    }

    /// Index a single file.
    ///
    /// Reads the file, parses it, extracts symbols, generates embeddings, and stores
    /// all data in the database. Updates statistics as it processes.
    ///
    /// # Arguments
    ///
    /// * `repo_id` - The repository ID this file belongs to
    /// * `file_path` - Path to the source file
    /// * `stats` - Statistics object to update
    ///
    /// # Errors
    ///
    /// Returns an error if file reading, parsing, or database operations fail.
    async fn index_file(
        &mut self,
        repo_id: i32,
        file_path: &Path,
        stats: &mut IndexingStats,
    ) -> Result<i32> {
        // Read file content with error context
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            crate::Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read file {}: {}", file_path.display(), e),
            ))
        })?;

        // Skip empty files
        if content.trim().is_empty() {
            return Err(crate::Error::Parse(format!(
                "File is empty: {}",
                file_path.display()
            )));
        }

        let language = CodeParser::detect_language(file_path);

        let (ast, hash) = self.parser.parse_file(file_path, &content)?;

        let file_id = self
            .db
            .add_file(
                repo_id,
                &file_path.to_string_lossy(),
                language.as_deref(),
                &content,
                &hash,
            )
            .await?;

        // Extract and index symbols
        if let Some(lang) = &language {
            *stats.files_by_language.entry(lang.clone()).or_insert(0) += 1;

            let symbols = self.parser.extract_symbols(&ast, lang);
            stats.total_symbols += symbols.len();

            for symbol in symbols {
                *stats
                    .symbols_by_kind
                    .entry(symbol.kind.clone())
                    .or_insert(0) += 1;
                self.index_symbol(file_id, &symbol).await?;
            }
        }

        Ok(file_id)
    }

    /// Index a single symbol.
    ///
    /// Generates an embedding for the symbol and stores it in the database.
    ///
    /// # Arguments
    ///
    /// * `file_id` - The file ID this symbol belongs to
    /// * `symbol` - The extracted symbol
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation or database insertion fails.
    async fn index_symbol(&self, file_id: i32, symbol: &Symbol) -> Result<i32> {
        // Validate symbol data
        if symbol.name.is_empty() {
            return Err(crate::Error::Parse(
                "Symbol name cannot be empty".to_string(),
            ));
        }

        let embedding = self.embedder.encode_symbol(
            &symbol.name,
            symbol.signature.as_deref(),
            symbol.docstring.as_deref(),
        )?;

        let symbol_id = self
            .db
            .add_symbol(
                file_id,
                &symbol.name,
                &symbol.kind,
                symbol.signature.as_deref(),
                symbol.docstring.as_deref(),
                symbol.start_line as i32,
                symbol.end_line as i32,
                &embedding,
            )
            .await?;

        Ok(symbol_id)
    }

    /// Find all source files in a repository.
    ///
    /// Walks the directory tree, skipping common ignore directories (node_modules, .git, etc.)
    /// and filtering for supported file types based on file extension.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Root path of the repository
    ///
    /// # Returns
    ///
    /// A vector of paths to source files that should be indexed.
    ///
    /// # Errors
    ///
    /// Returns an error if directory walking fails.
    fn find_source_files(&self, repo_path: &Path) -> Result<Vec<PathBuf>> {
        let skip_dirs = [
            ".git",
            "node_modules",
            "__pycache__",
            "venv",
            "env",
            ".venv",
            "dist",
            "build",
            "target",
            ".next",
            ".nuxt",
        ];

        let mut files = Vec::new();

        for entry in WalkDir::new(repo_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    !skip_dirs.iter().any(|&skip| name == skip)
                } else {
                    true
                }
            })
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if CodeParser::detect_language(path).is_some() {
                let metadata = path.metadata()?;
                if metadata.len() <= self.config.indexing.max_file_size as u64 {
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }
}
