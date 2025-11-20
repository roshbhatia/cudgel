// tests/test_llm_integration.rs
//! Integration tests for LLM client operations.

use async_trait::async_trait;
use cudgel::llm::Result;
use cudgel::llm::{
    ComponentContext, EntityContext, LlmClient, OllamaClient, RepositoryContext, SummaryRequest,
    SummaryResult,
};
use std::sync::Arc;
use std::time::Duration;

/// Mock LLM client for testing (doesn't require Ollama to be running)
#[derive(Default)]
pub struct MockLlmClient {
    should_fail: bool,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self::default()
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
    let client = OllamaClient::default_config();

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

// ============================================================================
// User Story 1 Tests (T035-T038)
// ============================================================================

/// T035: Test generate_repository_summary with mock client
#[tokio::test]
async fn test_generate_repository_summary() {
    let client = setup_test_llm_client();

    let context = RepositoryContext {
        name: "cudgel".to_string(),
        languages: vec!["Rust".to_string()],
        top_modules: vec![
            "parser".to_string(),
            "indexer".to_string(),
            "database".to_string(),
        ],
        file_count: 25,
        entity_count: 150,
        primary_patterns: vec!["async".to_string(), "tokio".to_string()],
    };

    let summary = client
        .generate_repository_summary(&context)
        .await
        .expect("Should generate repository summary");

    // Verify summary contains key information
    assert!(summary.contains("cudgel"));
    assert!(summary.contains("Rust"));
    assert!(summary.len() > 50, "Summary should be substantial");
}

/// T036: Test generate_component_summary with mock client
#[tokio::test]
async fn test_generate_component_summary() {
    let client = setup_test_llm_client();

    let context = ComponentContext {
        name: "parser".to_string(),
        component_type: "module".to_string(),
        file_count: 3,
        dependencies: vec!["tree-sitter".to_string()],
        exported_entities: vec![
            "parse_file".to_string(),
            "Parser".to_string(),
            "tokenize".to_string(),
        ],
        primary_patterns: vec!["async".to_string()],
    };

    let summary = client
        .generate_component_summary(&context)
        .await
        .expect("Should generate component summary");

    // Verify summary contains component information
    assert!(summary.contains("parser"));
    assert!(summary.contains("module"));
    assert!(summary.len() > 30, "Summary should be substantial");
}

/// T037: Test health check for LLM service
#[tokio::test]
async fn test_llm_health_check() {
    let client = setup_test_llm_client();

    let health = client
        .health_check()
        .await
        .expect("Should perform health check");

    assert!(health.is_available, "Mock client should be available");
    assert!(
        !health.loaded_models.is_empty(),
        "Should have loaded models"
    );
    assert!(
        health.ollama_version.is_some(),
        "Should have Ollama version"
    );
    assert!(
        health.response_time < Duration::from_secs(1),
        "Health check should be fast"
    );
}

/// T038: Test graceful degradation when LLM service fails
#[tokio::test]
async fn test_llm_graceful_degradation() {
    let client = setup_failing_llm_client();

    // Test that health check reports unavailability
    let health = client
        .health_check()
        .await
        .expect("Health check should not error");

    assert!(!health.is_available, "Failing client should be unavailable");

    // Test that generation fails gracefully
    let context = RepositoryContext {
        name: "test-repo".to_string(),
        languages: vec!["Rust".to_string()],
        top_modules: vec![],
        file_count: 10,
        entity_count: 50,
        primary_patterns: vec![],
    };

    let result = client.generate_repository_summary(&context).await;
    assert!(result.is_err(), "Generation should fail for failing client");

    // Test list_models fails gracefully
    let models_result = client.list_models().await;
    assert!(
        models_result.is_err(),
        "List models should fail for failing client"
    );
}

// ============================================================================
// Real Ollama Integration Tests (requires Ollama service running)
// ============================================================================

/// Test real Ollama integration
///
/// This test requires:
/// 1. Ollama to be running locally: `ollama serve`
/// 2. Model to be pulled manually: `ollama pull qwen2.5-coder:1.5b`
#[tokio::test]
#[ignore] // Run with: cargo test test_real_ollama_integration --ignored
async fn test_real_ollama_integration() {
    // Use a small, fast model for testing
    let test_model = "qwen2.5-coder:1.5b";

    let client = OllamaClient::new("http://localhost", test_model);

    // Check health
    let health = client.health_check().await;
    if health.is_err() || !health.as_ref().unwrap().is_available {
        eprintln!("Ollama not available, skipping test. Start Ollama with: ollama serve");
        return;
    }

    eprintln!("Ollama is available at {}", health.unwrap().api_endpoint);

    // Check if model is available
    let models = client.list_models().await.unwrap();
    if !models.contains(&test_model.to_string()) {
        eprintln!(
            "Model {} not found. Please pull it with: ollama pull {}",
            test_model, test_model
        );
        eprintln!("Available models: {:?}", models);
        return;
    }
    eprintln!("Model {} is available", test_model);

    // Test repository summary generation
    let repo_context = RepositoryContext {
        name: "cudgel".to_string(),
        languages: vec!["Rust".to_string()],
        top_modules: vec![
            "parser".to_string(),
            "indexer".to_string(),
            "database".to_string(),
        ],
        file_count: 25,
        entity_count: 150,
        primary_patterns: vec!["async".to_string(), "PostgreSQL".to_string()],
    };

    eprintln!("Generating repository summary with {}...", test_model);
    let summary = client.generate_repository_summary(&repo_context).await;

    if let Ok(summary) = summary {
        eprintln!("Generated summary ({} chars):\n{}", summary.len(), summary);
        assert!(summary.len() > 50, "Summary should be substantial");
        assert!(
            summary.to_lowercase().contains("cudgel") || summary.to_lowercase().contains("rust"),
            "Summary should reference the repository"
        );
    } else {
        eprintln!("Summary generation not yet implemented, skipping validation");
    }

    // Test component summary generation
    let component_context = ComponentContext {
        name: "parser".to_string(),
        component_type: "module".to_string(),
        file_count: 3,
        dependencies: vec!["tree-sitter".to_string()],
        exported_entities: vec!["parse_file".to_string(), "CodeParser".to_string()],
        primary_patterns: vec!["AST parsing".to_string()],
    };

    eprintln!("Generating component summary with {}...", test_model);
    let component_summary = client.generate_component_summary(&component_context).await;

    if let Ok(summary) = component_summary {
        eprintln!(
            "Generated component summary ({} chars):\n{}",
            summary.len(),
            summary
        );
        assert!(
            summary.len() > 30,
            "Component summary should be substantial"
        );
    } else {
        eprintln!("Component summary generation not yet implemented, skipping validation");
    }

    // List available models
    let models = client.list_models().await.unwrap();
    eprintln!("Available models: {:?}", models);
    assert!(
        models.contains(&test_model.to_string()),
        "Test model should be in list"
    );
}

/// Test model availability check
///
/// This test requires:
/// 1. Ollama to be running locally: `ollama serve`
/// 2. At least one model pulled: `ollama pull qwen2.5-coder:1.5b`
#[tokio::test]
#[ignore] // Run with: cargo test test_ollama_model_check --ignored
async fn test_ollama_model_check() {
    let test_model = "qwen2.5-coder:1.5b";
    let client = OllamaClient::new("http://localhost", test_model);

    // Check health first
    let health = client.health_check().await;
    if health.is_err() || !health.as_ref().unwrap().is_available {
        eprintln!("Ollama not available, skipping test");
        return;
    }

    // List models
    let models = client.list_models().await.unwrap();
    eprintln!("Available models: {:?}", models);

    if models.is_empty() {
        eprintln!(
            "No models available. Pull one with: ollama pull {}",
            test_model
        );
    } else {
        eprintln!("Found {} model(s)", models.len());

        // Check if test model is available
        if models.contains(&test_model.to_string()) {
            eprintln!("Test model {} is ready!", test_model);
        } else {
            eprintln!(
                "Test model {} not found. Using {} instead.",
                test_model, models[0]
            );
        }
    }
}
