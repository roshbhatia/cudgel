//! Code indexing functionality

use crate::{
    database::Database,
    embeddings::EmbeddingGenerator,
    parser::{CodeParser, Symbol},
    Config, Result,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub struct Indexer {
    config: Arc<Config>,
    db: Arc<Database>,
    parser: CodeParser,
    embedder: Arc<EmbeddingGenerator>,
}

impl Indexer {
    pub fn new(config: Arc<Config>, db: Arc<Database>) -> Result<Self> {
        let embedder = Arc::new(EmbeddingGenerator::new(config.clone())?);

        Ok(Indexer {
            config,
            db,
            parser: CodeParser::new(),
            embedder,
        })
    }

    pub async fn index_repository(&mut self, repo_path: &Path) -> Result<i32> {
        let name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let repo_path_str = repo_path.to_string_lossy().to_string();

        // Add repository to database
        let repo_id = self.db.add_repository(&repo_path_str, name).await?;

        // Find all source files
        let files = self.find_source_files(repo_path)?;

        println!("Found {} files to index", files.len());

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        let mut indexed = 0;
        for file_path in files {
            match self.index_file(repo_id, &file_path).await {
                Ok(_) => {
                    indexed += 1;
                    pb.set_message(format!("Indexing: {}", file_path.display()));
                }
                Err(e) => {
                    eprintln!("Error indexing {:?}: {}", file_path, e);
                }
            }
            pb.inc(1);
        }

        pb.finish_with_message(format!("Indexed {} files", indexed));

        Ok(repo_id)
    }

    async fn index_file(&mut self, repo_id: i32, file_path: &Path) -> Result<i32> {
        let content = std::fs::read_to_string(file_path)?;
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
        if let Some(lang) = language {
            let symbols = self.parser.extract_symbols(&ast, &lang);

            for symbol in symbols {
                self.index_symbol(file_id, &symbol).await?;
            }
        }

        Ok(file_id)
    }

    async fn index_symbol(&self, file_id: i32, symbol: &Symbol) -> Result<i32> {
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
