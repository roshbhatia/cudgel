// src/llm/client.rs
//! LLM client implementation using Ollama.

use super::{LlmError, Result};
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
