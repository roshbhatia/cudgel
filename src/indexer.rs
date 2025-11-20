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
//! let config = Arc::new(Config::local()?);
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
    embeddings::EmbedderBackend,
    kg::{EntityExtractor, KgClient, PostgresKgClient},
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

/// File filtering configuration for selective indexing.
///
/// Provides glob pattern matching and language-based filtering to control
/// which files get indexed during repository processing.
///
/// # Examples
///
/// ```no_run
/// use cudgel::indexer::IndexFilter;
///
/// // Include only specific patterns
/// let filter = IndexFilter::new()
///     .with_include_patterns(vec!["src/**/*.rs".to_string(), "tests/**/*.rs".to_string()])
///     .with_exclude_patterns(vec!["**/target/**".to_string()])
///     .with_languages(vec!["rust".to_string()]);
///
/// // Filter supports checking individual files
/// assert!(filter.should_index_file(std::path::Path::new("src/main.rs")));
/// ```
#[derive(Debug, Clone, Default)]
pub struct IndexFilter {
    include_patterns: Option<Vec<String>>,
    exclude_patterns: Option<Vec<String>>,
    languages: Option<Vec<String>>,
}

impl IndexFilter {
    /// Create a new empty filter (no filtering applied).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set include glob patterns.
    ///
    /// Only files matching at least one include pattern will be indexed.
    /// If no include patterns are specified, all files are included by default.
    pub fn with_include_patterns(mut self, patterns: Vec<String>) -> Self {
        self.include_patterns = if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        };
        self
    }

    /// Set exclude glob patterns.
    ///
    /// Files matching any exclude pattern will be skipped.
    /// Exclude patterns take precedence over include patterns.
    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        };
        self
    }

    /// Set language filter.
    ///
    /// Only files in the specified languages will be indexed.
    /// If no languages are specified, all supported languages are indexed.
    pub fn with_languages(mut self, langs: Vec<String>) -> Self {
        self.languages = if langs.is_empty() { None } else { Some(langs) };
        self
    }

    /// Validate filter configuration.
    ///
    /// Checks that language names are valid and patterns are well-formed.
    ///
    /// # Errors
    ///
    /// Returns an error if any language is unsupported or patterns are invalid.
    pub fn validate(&self) -> Result<()> {
        const SUPPORTED_LANGUAGES: &[&str] = &[
            "python",
            "javascript",
            "typescript",
            "rust",
            "go",
            "c",
            "cpp",
            "java",
        ];

        // Validate languages
        if let Some(langs) = &self.languages {
            for lang in langs {
                if !SUPPORTED_LANGUAGES.contains(&lang.as_str()) {
                    return Err(crate::Error::Config(format!(
                        "Unsupported language: '{}'. Supported languages: {}",
                        lang,
                        SUPPORTED_LANGUAGES.join(", ")
                    )));
                }
            }
        }

        // Validate glob patterns by attempting to compile them
        if let Some(patterns) = &self.include_patterns {
            for pattern in patterns {
                glob::Pattern::new(pattern).map_err(|e| {
                    crate::Error::Config(format!("Invalid include pattern '{}': {}", pattern, e))
                })?;
            }
        }

        if let Some(patterns) = &self.exclude_patterns {
            for pattern in patterns {
                glob::Pattern::new(pattern).map_err(|e| {
                    crate::Error::Config(format!("Invalid exclude pattern '{}': {}", pattern, e))
                })?;
            }
        }

        Ok(())
    }

    /// Check if a file should be indexed based on this filter.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to check
    ///
    /// # Returns
    ///
    /// `true` if the file passes all filters and should be indexed, `false` otherwise.
    pub fn should_index_file(&self, file_path: &Path) -> bool {
        // Check language filter first (cheapest check)
        if let Some(langs) = &self.languages {
            if let Some(detected_lang) = CodeParser::detect_language(file_path) {
                if !langs.contains(&detected_lang) {
                    return false;
                }
            } else {
                // Unknown language, skip
                return false;
            }
        }

        let path_str = file_path.to_string_lossy();

        // Check exclude patterns (take precedence)
        if let Some(exclude) = &self.exclude_patterns {
            for pattern in exclude {
                if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                    if glob_pattern.matches(&path_str) {
                        return false;
                    }
                }
            }
        }

        // Check include patterns (if specified)
        if let Some(include) = &self.include_patterns {
            let mut matched = false;
            for pattern in include {
                if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                    if glob_pattern.matches(&path_str) {
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                return false;
            }
        }

        true
    }

    /// Get reference to include patterns.
    pub fn include_patterns(&self) -> Option<&Vec<String>> {
        self.include_patterns.as_ref()
    }

    /// Get reference to exclude patterns.
    pub fn exclude_patterns(&self) -> Option<&Vec<String>> {
        self.exclude_patterns.as_ref()
    }

    /// Get reference to languages filter.
    pub fn languages(&self) -> Option<&Vec<String>> {
        self.languages.as_ref()
    }

    /// Check if this filter has any active filters.
    pub fn is_empty(&self) -> bool {
        self.include_patterns.is_none()
            && self.exclude_patterns.is_none()
            && self.languages.is_none()
    }
}

/// The main indexing engine for Cudgel.
///
/// Handles walking directory trees, parsing source files, extracting symbols,
/// and storing indexed data in PostgreSQL.
pub struct Indexer {
    config: Arc<Config>,
    db: Arc<Database>,
    parser: CodeParser,
    embedder: Arc<EmbedderBackend>,
    kg_client: Option<PostgresKgClient>,
    entity_extractor: Option<EntityExtractor>,
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
        let embedder = Arc::new(EmbedderBackend::from_config(&config)?);

        Ok(Indexer {
            config,
            db,
            parser: CodeParser::new(),
            embedder,
            kg_client: None,
            entity_extractor: None,
        })
    }

    /// Create a new indexer instance with knowledge graph support.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `db` - Database connection pool
    /// * `enable_kg` - Whether to enable knowledge graph functionality
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generator or KG client cannot be initialized.
    pub fn new_with_kg(config: Arc<Config>, db: Arc<Database>, enable_kg: bool) -> Result<Self> {
        let embedder = Arc::new(EmbedderBackend::from_config(&config)?);

        let (kg_client, entity_extractor) = if enable_kg {
            let kg_client = PostgresKgClient::new(db.clone());
            let entity_extractor = EntityExtractor::new(0); // Will be set when repository is created
            (Some(kg_client), Some(entity_extractor))
        } else {
            (None, None)
        };

        Ok(Indexer {
            config,
            db,
            parser: CodeParser::new(),
            embedder,
            kg_client,
            entity_extractor,
        })
    }

    fn get_git_repo_name(repo_path: &Path) -> String {
        std::process::Command::new("git")
            .args([
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
            .args(["-C", &repo_path.to_string_lossy(), "ls-files"])
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

    /// Index a repository with file filtering.
    ///
    /// Similar to `index_repository()` but applies include/exclude patterns and
    /// language filters before indexing files.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository to index
    /// * `filter` - File filtering configuration
    ///
    /// # Returns
    ///
    /// Tuple of (repository_id, indexing_statistics)
    ///
    /// # Errors
    ///
    /// Returns an error if path validation, git operations, or database operations fail.
    pub async fn index_repository_with_filter(
        &mut self,
        repo_path: &Path,
        filter: &IndexFilter,
    ) -> Result<(i32, IndexingStats)> {
        // Validate path
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

        // Validate filter
        filter.validate()?;

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
        let mut skipped_by_filter = 0;

        all_files.retain(|path| {
            if !path.is_file() {
                return false;
            }

            // Apply filter first (cheapest check)
            if !filter.should_index_file(path) {
                skipped_by_filter += 1;
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
        if skipped_by_filter > 0 {
            println!(
                "Skipped {} files due to include/exclude/language filters",
                skipped_by_filter
            );
        }
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

            // Extract KG entities if KG client is available
            if let (Some(kg_client), Some(ref mut entity_extractor)) =
                (&self.kg_client, &mut self.entity_extractor)
            {
                let entities = entity_extractor
                    .symbols_to_entities(symbols.clone(), &path_str, lang, kg_client)
                    .await?;
                for entity in entities {
                    if let Err(e) = kg_client.create_entity(entity).await {
                        eprintln!("Warning: Failed to create KG entity: {}", e);
                    }
                }
            }

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

        // Build text for embedding: name + signature + docstring
        let text = format!(
            "{}{}{}",
            symbol.name,
            signature.map(|s| format!(" {}", s)).unwrap_or_default(),
            docstring.map(|d| format!(" {}", d)).unwrap_or_default()
        );

        let embedding = self.embedder.encode(&text)?;

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
