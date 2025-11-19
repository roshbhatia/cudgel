// tests/test_llm_integration.rs
//! Integration tests for LLM client operations.

use cudgel::llm::{
    ComponentContext, EntityContext, LlmClient, OllamaClient, RepositoryContext, SummaryRequest,
    SummaryResult,
};
use cudgel::llm::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Mock LLM client for testing (doesn't require Ollama to be running)
pub struct MockLlmClient {
    should_fail: bool,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    pub fn with_failures() -> Self {
        Self { should_fail: true }
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn generate_repository_summary(&self, context: &RepositoryContext) -> Result<String> {
        if self.should_fail {
            return Err(cudgel::llm::LlmError::Generation(
                "Mock generation failure".to_string(),
            ));
        }

        Ok(format!(
            "Mock repository summary for {}: A {} repository with {} files and {} entities.",
            context.name,
            context.languages.join("/"),
            context.file_count,
            context.entity_count
        ))
    }

    async fn generate_component_summary(&self, context: &ComponentContext) -> Result<String> {
        if self.should_fail {
            return Err(cudgel::llm::LlmError::Generation(
                "Mock generation failure".to_string(),
            ));
        }

        Ok(format!(
            "Mock component summary for {}: A {} component with {} files.",
            context.name, context.component_type, context.file_count
        ))
    }

    async fn generate_entity_summary(&self, context: &EntityContext) -> Result<String> {
        if self.should_fail {
            return Err(cudgel::llm::LlmError::Generation(
                "Mock generation failure".to_string(),
            ));
        }

        Ok(format!(
            "Mock entity summary for {}: A {} defined in {}.",
            context.name, context.entity_type, context.file_path
        ))
    }

    async fn analyze_pattern(&self, pattern: &str, _entities: &[String]) -> Result<String> {
        if self.should_fail {
            return Err(cudgel::llm::LlmError::Generation(
                "Mock generation failure".to_string(),
            ));
        }

        Ok(format!("Mock pattern analysis for pattern: {}", pattern))
    }

    async fn generate_summaries_batch(
        &self,
        requests: Vec<SummaryRequest>,
        _concurrency: usize,
    ) -> Result<Vec<SummaryResult>> {
        let mut results = Vec::new();

        for request in requests {
            let summary = match &request {
                SummaryRequest::Repository(ctx) => self.generate_repository_summary(ctx).await,
                SummaryRequest::Component(ctx) => self.generate_component_summary(ctx).await,
                SummaryRequest::Entity(ctx) => self.generate_entity_summary(ctx).await,
            };

            results.push(SummaryResult {
                request: request.clone(),
                summary: summary.ok(),
                error: None,
                generation_time: Duration::from_millis(10),
                token_count: Some(50),
            });
        }

        Ok(results)
    }

    async fn health_check(&self) -> Result<cudgel::llm::ServiceHealth> {
        Ok(cudgel::llm::ServiceHealth {
            is_available: !self.should_fail,
            ollama_version: Some("mock-1.0.0".to_string()),
            loaded_models: vec!["mock-model".to_string()],
            api_endpoint: "http://mock:11434".to_string(),
            response_time: Duration::from_millis(5),
        })
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        if self.should_fail {
            return Err(cudgel::llm::LlmError::Connection(
                "Mock connection failure".to_string(),
            ));
        }

        Ok(vec!["mock-model".to_string()])
    }

    fn set_temperature(&mut self, _temperature: f32) {}
    fn set_timeout(&mut self, _timeout: Duration) {}
    fn set_model(&mut self, _model: String) {}
}

/// Setup a test LLM client (mock version for unit tests)
///
/// Always succeeds and returns a mock client that doesn't require Ollama.
pub fn setup_test_llm_client() -> Arc<dyn LlmClient> {
    Arc::new(MockLlmClient::new())
}

/// Setup a test LLM client that simulates failures
pub fn setup_failing_llm_client() -> Arc<dyn LlmClient> {
    Arc::new(MockLlmClient::with_failures())
}

/// Try to setup a real Ollama client (for integration tests)
///
/// Returns `Some(Arc<dyn LlmClient>)` if Ollama is available, `None` otherwise.
pub async fn setup_real_ollama_client() -> Option<Arc<dyn LlmClient>> {
    let client = OllamaClient::default();
    
    match client.health_check().await {
        Ok(health) if health.is_available => Some(Arc::new(client)),
        _ => {
            eprintln!("Ollama not available, skipping integration test");
            None
        }
    }
}

#[tokio::test]
async fn test_mock_llm_client() {
    let client = setup_test_llm_client();
    
    let context = RepositoryContext {
        name: "test-repo".to_string(),
        languages: vec!["Rust".to_string()],
        top_modules: vec!["core".to_string()],
        file_count: 42,
        entity_count: 150,
        primary_patterns: vec!["async".to_string()],
    };
    
    let summary = client.generate_repository_summary(&context).await;
    assert!(summary.is_ok());
    assert!(summary.unwrap().contains("test-repo"));
}

// Placeholder tests for User Story 1 (to be implemented in Phase 3)

#[tokio::test]
#[ignore] // Enable when T035 is implemented
async fn test_generate_repository_summary() {
    todo!("T035: Implement test_generate_repository_summary")
}

#[tokio::test]
#[ignore] // Enable when T036 is implemented
async fn test_generate_component_summary() {
    todo!("T036: Implement test_generate_component_summary")
}

#[tokio::test]
#[ignore] // Enable when T037 is implemented
async fn test_llm_health_check() {
    todo!("T037: Implement test_llm_health_check")
}

#[tokio::test]
#[ignore] // Enable when T038 is implemented
async fn test_llm_graceful_degradation() {
    todo!("T038: Implement test_llm_graceful_degradation")
}
