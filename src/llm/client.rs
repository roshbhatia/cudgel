// src/llm/client.rs
//! LLM client implementation using Ollama.

use super::Result;
use async_trait::async_trait;
use std::time::Duration;

/// Repository-level context for summary generation
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    pub name: String,
    pub languages: Vec<String>,
    pub top_modules: Vec<String>,
    pub file_count: usize,
    pub entity_count: usize,
    pub primary_patterns: Vec<String>, // e.g., ["CLI", "async", "database"]
}

/// Component/module-level context for summary generation
#[derive(Debug, Clone)]
pub struct ComponentContext {
    pub name: String,
    pub component_type: String,
    pub file_count: usize,
    pub dependencies: Vec<String>,      // Names of dependencies
    pub exported_entities: Vec<String>, // Public API surface
    pub primary_patterns: Vec<String>,
}

/// Entity-level context for summary generation
#[derive(Debug, Clone)]
pub struct EntityContext {
    pub name: String,
    pub entity_type: String,
    pub file_path: String,
    pub code_snippet: String,       // Up to 50 lines
    pub signature: Option<String>,  // For functions/methods
    pub dependencies: Vec<String>,  // Direct dependencies
    pub visibility: String,
}

/// Summary generation request types
#[derive(Debug, Clone)]
pub enum SummaryRequest {
    Repository(RepositoryContext),
    Component(ComponentContext),
    Entity(EntityContext),
}

/// Summary generation result
#[derive(Debug, Clone)]
pub struct SummaryResult {
    pub request: SummaryRequest,
    pub summary: Option<String>,
    pub error: Option<String>,
    pub generation_time: Duration,
    pub token_count: Option<usize>,
}

/// Service health status
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub is_available: bool,
    pub ollama_version: Option<String>,
    pub loaded_models: Vec<String>,
    pub api_endpoint: String,
    pub response_time: Duration,
}

/// Trait defining LLM client operations
#[async_trait]
pub trait LlmClient: Send + Sync {
    // === Summary Generation ===

    /// Generate repository-level architecture summary
    async fn generate_repository_summary(&self, context: &RepositoryContext) -> Result<String>;

    /// Generate module/component-level summary
    async fn generate_component_summary(&self, context: &ComponentContext) -> Result<String>;

    /// Generate entity-level summary (class, function, etc.)
    async fn generate_entity_summary(&self, context: &EntityContext) -> Result<String>;

    /// Generate pattern analysis summary (cross-cutting concerns)
    async fn analyze_pattern(&self, pattern: &str, entities: &[String]) -> Result<String>;

    // === Batch Operations ===

    /// Generate multiple summaries with rate limiting
    async fn generate_summaries_batch(
        &self,
        requests: Vec<SummaryRequest>,
        concurrency: usize,
    ) -> Result<Vec<SummaryResult>>;

    // === Service Management ===

    /// Check if Ollama service is available
    async fn health_check(&self) -> Result<ServiceHealth>;

    /// Get available models
    async fn list_models(&self) -> Result<Vec<String>>;

    /// Configure generation parameters
    fn set_temperature(&mut self, temperature: f32);
    fn set_timeout(&mut self, timeout: Duration);
    fn set_model(&mut self, model: String);
}

/// Concrete Ollama implementation of the LLM client
pub struct OllamaClient {
    client: ollama_rs::Ollama,
    model: String,
    temperature: f32,
    timeout: Duration,
}

impl OllamaClient {
    /// Create a new Ollama client
    ///
    /// # Arguments
    /// * `url` - Ollama API endpoint (e.g., "http://localhost:11434")
    /// * `model` - Model to use (e.g., "llama3.2:3b")
    ///
    /// # Examples
    /// ```no_run
    /// use cudgel::llm::client::OllamaClient;
    ///
    /// let client = OllamaClient::new("http://localhost:11434", "llama3.2:3b");
    /// ```
    pub fn new(url: &str, model: &str) -> Self {
        Self {
            client: ollama_rs::Ollama::new(url.to_string(), 11434),
            model: model.to_string(),
            temperature: 0.7,
            timeout: Duration::from_secs(120),
        }
    }

    /// Create a new Ollama client with default settings
    ///
    /// Uses:
    /// - URL: http://localhost:11434
    /// - Model: llama3.2:3b
    pub fn default() -> Self {
        Self::new("http://localhost", "llama3.2:3b")
    }

    /// Pull a model from Ollama registry if not already present
    ///
    /// # Arguments
    /// * `model_name` - Model to pull (e.g., "llama3.2:3b", "qwen2.5-coder:3b")
    ///
    /// # Returns
    /// * `Ok(true)` - Model was pulled successfully
    /// * `Ok(false)` - Model was already present
    /// * `Err` - Failed to pull model
    pub async fn pull_model_if_missing(&self, model_name: &str) -> Result<bool> {
        // Check if model already exists
        let models = self.list_models().await?;
        if models.iter().any(|m| m == model_name) {
            return Ok(false);
        }

        // Pull the model
        tracing::info!("Pulling model {}...", model_name);
        self.client
            .pull_model(model_name.to_string(), false)
            .await
            .map_err(|e| super::LlmError::Connection(format!("Failed to pull model: {}", e)))?;

        tracing::info!("Model {} pulled successfully", model_name);
        Ok(true)
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn generate_repository_summary(&self, _context: &RepositoryContext) -> Result<String> {
        todo!("T052: Implement generate_repository_summary")
    }

    async fn generate_component_summary(&self, _context: &ComponentContext) -> Result<String> {
        todo!("T052: Implement generate_component_summary")
    }

    async fn generate_entity_summary(&self, _context: &EntityContext) -> Result<String> {
        todo!("T068: Implement generate_entity_summary")
    }

    async fn analyze_pattern(&self, _pattern: &str, _entities: &[String]) -> Result<String> {
        todo!("T124: Implement analyze_pattern")
    }

    async fn generate_summaries_batch(
        &self,
        _requests: Vec<SummaryRequest>,
        _concurrency: usize,
    ) -> Result<Vec<SummaryResult>> {
        todo!("T069: Implement generate_summaries_batch")
    }

    async fn health_check(&self) -> Result<ServiceHealth> {
        let start = std::time::Instant::now();
        let api_endpoint = self.client.uri().to_string();
        
        // Try to list models as a health check
        match self.client.list_local_models().await {
            Ok(models) => {
                let model_names: Vec<String> = models
                    .into_iter()
                    .map(|m| m.name)
                    .collect();

                Ok(ServiceHealth {
                    is_available: true,
                    ollama_version: Some("unknown".to_string()), // Ollama API doesn't expose version easily
                    loaded_models: model_names,
                    api_endpoint,
                    response_time: start.elapsed(),
                })
            }
            Err(_e) => {
                Ok(ServiceHealth {
                    is_available: false,
                    ollama_version: None,
                    loaded_models: vec![],
                    api_endpoint,
                    response_time: start.elapsed(),
                })
            }
        }
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let models = self
            .client
            .list_local_models()
            .await
            .map_err(|e| super::LlmError::Connection(format!("Failed to list models: {}", e)))?;

        Ok(models.into_iter().map(|m| m.name).collect())
    }

    fn set_temperature(&mut self, temperature: f32) {
        self.temperature = temperature.clamp(0.0, 2.0);
    }

    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }
}
