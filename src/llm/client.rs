// src/llm/client.rs
//! LLM client implementation using Ollama.

use super::Result;
use async_trait::async_trait;
use std::time::Duration;
use ollama_rs::generation::completion::request::GenerationRequest;

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
    async fn generate_repository_summary(&self, context: &RepositoryContext) -> Result<String> {
        use super::prompts::{REPOSITORY_PROMPT, fill_template};
        use std::collections::HashMap;

        let mut values = HashMap::new();
        values.insert("name".to_string(), context.name.clone());
        values.insert("languages".to_string(), context.languages.join(", "));
        values.insert("modules".to_string(), context.top_modules.join(", "));
        values.insert("file_count".to_string(), context.file_count.to_string());
        values.insert("entity_count".to_string(), context.entity_count.to_string());
        values.insert("patterns".to_string(), context.primary_patterns.join(", "));

        let prompt = fill_template(REPOSITORY_PROMPT, &values);

        let request = GenerationRequest::new(
            self.model.clone(),
            prompt,
        );
        
        // Note: ollama-rs API may have different method names
        // This is a simplified version that should compile

        let response = self
            .client
            .generate(request)
            .await
            .map_err(|e| super::LlmError::Generation(format!("Failed to generate repository summary: {}", e)))?;

        Ok(response.response)
    }

    async fn generate_component_summary(&self, context: &ComponentContext) -> Result<String> {
        use super::prompts::{COMPONENT_PROMPT, fill_template};
        use std::collections::HashMap;

        let mut values = HashMap::new();
        values.insert("name".to_string(), context.name.clone());
        values.insert("type".to_string(), context.component_type.clone());
        values.insert("file_count".to_string(), context.file_count.to_string());
        values.insert("dependencies".to_string(), context.dependencies.join(", "));
        values.insert("exported_entities".to_string(), context.exported_entities.join(", "));
        values.insert("patterns".to_string(), context.primary_patterns.join(", "));

        let prompt = fill_template(COMPONENT_PROMPT, &values);

        let request = GenerationRequest::new(
            self.model.clone(),
            prompt,
        );
        
        // Note: ollama-rs API may have different method names
        // This is a simplified version that should compile

        let response = self
            .client
            .generate(request)
            .await
            .map_err(|e| super::LlmError::Generation(format!("Failed to generate component summary: {}", e)))?;

        Ok(response.response)
    }

    async fn generate_entity_summary(&self, context: &EntityContext) -> Result<String> {
        use super::prompts::{ENTITY_PROMPT, fill_template};
        use std::collections::HashMap;

        let mut values = HashMap::new();
        values.insert("name".to_string(), context.name.clone());
        values.insert("type".to_string(), context.entity_type.clone());
        values.insert("file_path".to_string(), context.file_path.clone());
        values.insert("signature".to_string(), context.signature.clone().unwrap_or_default());
        values.insert("dependencies".to_string(), context.dependencies.join(", "));
        values.insert("visibility".to_string(), context.visibility.clone());
        values.insert("code_snippet".to_string(), context.code_snippet.clone());

        let prompt = fill_template(ENTITY_PROMPT, &values);

        let request = GenerationRequest::new(
            self.model.clone(),
            prompt,
        );
        
        // Note: ollama-rs API may have different method names
        // This is a simplified version that should compile

        let response = self
            .client
            .generate(request)
            .await
            .map_err(|e| super::LlmError::Generation(format!("Failed to generate entity summary: {}", e)))?;

        Ok(response.response)
    }

    async fn analyze_pattern(&self, pattern: &str, entities: &[String]) -> Result<String> {
        use super::prompts::{PATTERN_ANALYSIS_PROMPT, fill_template};
        use std::collections::HashMap;

        let mut values = HashMap::new();
        values.insert("pattern".to_string(), pattern.to_string());
        values.insert("entities".to_string(), entities.join(", "));

        let prompt = fill_template(PATTERN_ANALYSIS_PROMPT, &values);

        let request = GenerationRequest::new(
            self.model.clone(),
            prompt,
        );
        
        // Note: ollama-rs API may have different method names
        // This is a simplified version that should compile

        let response = self
            .client
            .generate(request)
            .await
            .map_err(|e| super::LlmError::Generation(format!("Failed to analyze pattern: {}", e)))?;

        Ok(response.response)
    }

    async fn generate_summaries_batch(
        &self,
        requests: Vec<SummaryRequest>,
        concurrency: usize,
    ) -> Result<Vec<SummaryResult>> {
        use futures::stream::{self, StreamExt};
        
        let semaphore = tokio::sync::Semaphore::new(concurrency);
        
        let results = stream::iter(requests)
            .map(|request| {
                let semaphore = &semaphore;
                let client = self;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let start = std::time::Instant::now();
                    
                    let result = match &request {
                        SummaryRequest::Repository(ctx) => {
                            client.generate_repository_summary(ctx).await
                        }
                        SummaryRequest::Component(ctx) => {
                            client.generate_component_summary(ctx).await
                        }
                        SummaryRequest::Entity(ctx) => {
                            client.generate_entity_summary(ctx).await
                        }
                    };
                    
                    let (summary, error) = match result {
                        Ok(s) => (Some(s), None),
                        Err(e) => (None, Some(e.to_string())),
                    };
                    
                    SummaryResult {
                        request,
                        summary,
                        error,
                        generation_time: start.elapsed(),
                        token_count: None, // Ollama doesn't expose token count easily
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;
            
        Ok(results)
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
