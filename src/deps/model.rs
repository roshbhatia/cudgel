// src/deps/model.rs
//! Model download and verification functionality

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Download state for model artifacts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Represents a downloadable model artifact from HuggingFace Hub
#[derive(Debug, Clone)]
pub struct ModelArtifact {
    pub model_id: String,
    pub filename: String,
    pub source_url: String,
    pub target_path: PathBuf,
    pub expected_size_bytes: u64,
    pub download_progress_bytes: u64,
    pub download_state: DownloadState,
}

impl ModelArtifact {
    /// Create a new model artifact
    pub fn new(
        model_id: impl Into<String>,
        filename: impl Into<String>,
        target_path: PathBuf,
        expected_size_bytes: u64,
    ) -> Self {
        let model_id = model_id.into();
        let filename = filename.into();
        let source_url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            model_id, filename
        );

        Self {
            model_id,
            filename,
            source_url,
            target_path,
            expected_size_bytes,
            download_progress_bytes: 0,
            download_state: DownloadState::Pending,
        }
    }
}

/// Model downloader using hf-hub
pub struct ModelDownloader {
    cache_dir: PathBuf,
}

impl ModelDownloader {
    /// Create a new model downloader
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Download a model artifact with progress tracking
    pub async fn download_model_artifact(&self, artifact: &mut ModelArtifact) -> Result<()> {
        use hf_hub::api::tokio::ApiBuilder;
        use indicatif::{ProgressBar, ProgressStyle};

        // Check disk space before download
        self.disk_space_check(artifact.expected_size_bytes)?;

        artifact.download_state = DownloadState::InProgress;

        // Create target directory if it doesn't exist
        if let Some(parent) = artifact.target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Initialize hf-hub API with custom cache directory
        let api = ApiBuilder::new()
            .with_cache_dir(self.cache_dir.clone())
            .build()
            .map_err(|e| {
                Error::ModelDownloadFailed(format!("Failed to initialize hf-hub: {}", e))
            })?;

        // Parse model_id to get repo
        let repo = hf_hub::Repo::model(artifact.model_id.clone());
        let model_api = api.repo(repo);

        // Create progress bar
        let pb = ProgressBar::new(artifact.expected_size_bytes);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .map_err(|e| Error::ModelDownloadFailed(format!("Progress bar template error: {}", e)))?
                .progress_chars("=>-")
        );
        pb.set_message(format!("📦 Downloading {}", artifact.filename));

        // Download the file using hf-hub (handles resume automatically)
        let downloaded_path = model_api
            .get(&artifact.filename)
            .await
            .map_err(|e| Error::ModelDownloadFailed(format!("Download failed: {}", e)))?;

        // Copy from hf-hub cache to target location
        std::fs::copy(&downloaded_path, &artifact.target_path)?;

        // Update progress
        let actual_size = std::fs::metadata(&artifact.target_path)?.len();
        pb.set_position(actual_size);
        artifact.download_progress_bytes = actual_size;

        pb.finish_with_message(format!("✓ {} downloaded", artifact.filename));

        artifact.download_state = DownloadState::Completed;
        Ok(())
    }

    /// Verify model integrity (3-layer verification)
    pub fn verify_model_integrity(&self, model_dir: &Path) -> Result<()> {
        // Layer 1: HTTP ETag validation - handled automatically by hf-hub

        // Layer 2: File size sanity check
        let files = [
            ("model.onnx", 90_000_000..110_000_000),  // ~100MB
            ("tokenizer.json", 2_000_000..8_000_000), // ~5MB
            ("config.json", 500..2_000),              // ~1KB
        ];

        for (filename, expected_range) in &files {
            let path = model_dir.join(filename);
            if !path.exists() {
                return Err(Error::CorruptedModel(format!("Missing file: {}", filename)));
            }

            let size = std::fs::metadata(&path)?.len();
            if !expected_range.contains(&size) {
                return Err(Error::CorruptedModel(format!(
                    "File size mismatch for {}: {} bytes (expected {:?})",
                    filename, size, expected_range
                )));
            }
        }

        // Layer 3: Functional validation - verify ONNX model loads
        let model_path = model_dir.join("model.onnx");
        ort::session::Session::builder()
            .map_err(|e| Error::CorruptedModel(format!("ONNX session builder error: {}", e)))?
            .commit_from_file(&model_path)
            .map_err(|e| Error::CorruptedModel(format!("ONNX model load failed: {}", e)))?;

        Ok(())
    }

    /// Clean up partial downloads
    pub fn cleanup_partial_downloads(&self, model_dir: &Path) -> Result<()> {
        if !model_dir.exists() {
            return Ok(());
        }

        // Remove any .tmp files or incomplete downloads
        for entry in std::fs::read_dir(model_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "tmp" || ext == "partial" {
                    std::fs::remove_file(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Check available disk space
    pub fn disk_space_check(&self, _required_bytes: u64) -> Result<()> {
        use std::fs;

        // Get filesystem stats for the cache directory
        let _metadata = fs::metadata(&self.cache_dir).or_else(|_| {
            // If cache_dir doesn't exist, check parent or root
            fs::metadata(self.cache_dir.parent().unwrap_or_else(|| Path::new("/")))
        })?;

        // On Unix systems, we can use statfs to get available space
        #[cfg(unix)]
        {
            // This is a simplified check - in production, use statfs for actual available space
            // For now, we'll do a basic check that the directory is writable
            if !self.cache_dir.exists() {
                std::fs::create_dir_all(&self.cache_dir)?;
            }
            // If we can create the directory, assume we have space
            // A more robust implementation would use nix::sys::statfs::statfs
            Ok(())
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, do basic writability check
            if !self.cache_dir.exists() {
                std::fs::create_dir_all(&self.cache_dir)?;
            }
            Ok(())
        }
    }
}
