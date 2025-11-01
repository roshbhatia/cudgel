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

    fn get_git_repo_name(repo_path: &Path) -> String {
        std::process::Command::new("git")
            .args(&[
                "-C",
                &repo_path.to_string_lossy(),
                "remote",
                "get-url",
                "origin",
            ])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok().map(|s| {
                        s.trim()
                            .rsplit('/')
                            .next()
                            .unwrap_or("unknown")
                            .trim_end_matches(".git")
                            .to_string()
                    })
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                repo_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            })
    }

    fn get_git_tracked_files(repo_path: &Path) -> Result<Vec<PathBuf>> {
        let output = std::process::Command::new("git")
            .args(&["-C", &repo_path.to_string_lossy(), "ls-files"])
            .output()
            .map_err(|e| crate::Error::Other(format!("Failed to run git ls-files: {}", e)))?;

        if !output.status.success() {
            return Err(crate::Error::Other(
                "git ls-files failed - is this a git repository?".to_string(),
            ));
        }

        let files_str = String::from_utf8(output.stdout)
            .map_err(|e| crate::Error::Other(format!("Invalid UTF-8 from git: {}", e)))?;

        Ok(files_str
            .lines()
            .map(|line| repo_path.join(line.trim()))
            .collect())
    }

    pub async fn index_repository(&mut self, repo_path: &Path) -> Result<(i32, IndexingStats)> {
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

        let absolute_path = repo_path.canonicalize().map_err(|e| {
            crate::Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to get absolute path: {}", e),
            ))
        })?;

        let name = Self::get_git_repo_name(&absolute_path);
        let repo_path_str = absolute_path.to_string_lossy().to_string();

        let repo_id = self.db.add_repository(&repo_path_str, &name).await?;

        self.db.delete_repository_symbols(repo_id).await?;

        let mut all_files = Self::get_git_tracked_files(&absolute_path)?;

        // Track skipped files for reporting
        let mut skipped_large = 0;
        let mut skipped_unsupported = 0;
        let mut skipped_no_metadata = 0;

        all_files.retain(|path| {
            if !path.is_file() {
                return false;
            }

            // Check if language is supported
            if CodeParser::detect_language(path).is_none() {
                skipped_unsupported += 1;
                return false;
            }

            // Check file size
            match path.metadata() {
                Ok(metadata) => {
                    let size = metadata.len();
                    if size > self.config.indexing.max_file_size as u64 {
                        skipped_large += 1;
                        eprintln!(
                            "Skipping large file: {} ({} bytes, max: {} bytes)",
                            path.display(),
                            size,
                            self.config.indexing.max_file_size
                        );
                        false
                    } else if size == 0 {
                        // Skip empty files
                        false
                    } else {
                        true
                    }
                }
                Err(_) => {
                    skipped_no_metadata += 1;
                    false
                }
            }
        });

        let files = all_files;
        let total_files = files.len();

        println!("Found {} files to index", total_files);
        if skipped_large > 0 {
            println!(
                "Skipped {} files exceeding max size ({} bytes)",
                skipped_large, self.config.indexing.max_file_size
            );
        }
        if skipped_unsupported > 0 {
            println!(
                "Skipped {} files with unsupported language",
                skipped_unsupported
            );
        }
        if skipped_no_metadata > 0 {
            println!(
                "Skipped {} files with inaccessible metadata",
                skipped_no_metadata
            );
        }

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
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            crate::Error::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read file {}: {}", file_path.display(), e),
            ))
        })?;

        if content.trim().is_empty() {
            return Err(crate::Error::Parse(format!(
                "File is empty: {}",
                file_path.display()
            )));
        }

        let language = CodeParser::detect_language(file_path);
        let (ast, hash) = self.parser.parse_file(file_path, &content)?;

        let path_str = file_path.to_string_lossy();
        let existing_hash = self.db.get_file_hash(repo_id, &path_str).await?;

        if let Some(old_hash) = existing_hash {
            if old_hash == hash {
                if let Some(file_id) = self.db.get_file_id(repo_id, &path_str).await? {
                    return Ok(file_id);
                }
            } else if let Some(file_id) = self.db.get_file_id(repo_id, &path_str).await? {
                self.db.delete_file_symbols(file_id).await?;
            }
        }

        let file_id = self
            .db
            .add_file(repo_id, &path_str, language.as_deref(), &content, &hash)
            .await?;

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

        // Validate symbol name length (prevent extremely long names)
        const MAX_SYMBOL_NAME_LENGTH: usize = 1000;
        if symbol.name.len() > MAX_SYMBOL_NAME_LENGTH {
            return Err(crate::Error::Parse(format!(
                "Symbol name too long: {} characters (max: {})",
                symbol.name.len(),
                MAX_SYMBOL_NAME_LENGTH
            )));
        }

        // Validate line numbers are reasonable (end_line should be >= start_line)
        if symbol.end_line < symbol.start_line {
            return Err(crate::Error::Parse(format!(
                "Invalid line numbers for symbol '{}': start={}, end={}",
                symbol.name, symbol.start_line, symbol.end_line
            )));
        }

        // Limit signature and docstring lengths to prevent memory issues
        const MAX_TEXT_LENGTH: usize = 10000;
        let signature = symbol.signature.as_ref().map(|s| {
            if s.len() > MAX_TEXT_LENGTH {
                &s[..MAX_TEXT_LENGTH]
            } else {
                s.as_str()
            }
        });

        let docstring = symbol.docstring.as_ref().map(|s| {
            if s.len() > MAX_TEXT_LENGTH {
                &s[..MAX_TEXT_LENGTH]
            } else {
                s.as_str()
            }
        });

        let embedding = self
            .embedder
            .encode_symbol(&symbol.name, signature, docstring)?;

        let symbol_id = self
            .db
            .add_symbol(
                file_id,
                &symbol.name,
                &symbol.kind,
                signature,
                docstring,
                symbol.start_line as i32,
                symbol.end_line as i32,
                &embedding,
            )
            .await?;

        Ok(symbol_id)
    }
}
