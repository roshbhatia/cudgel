# Contract: LLM Client Interface

**Feature**: 003-knowledge-graph  
**Date**: 2025-11-19

## Overview

This contract defines the interface for generating natural language summaries of code architecture using Ollama. The interface abstracts LLM interactions and provides type-safe summary generation.

---

## Interface Definition

### LlmClient

The primary interface for LLM-based summary generation.

```rust
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait LlmClient: Send + Sync {
    // === Summary Generation ===
    
    /// Generate repository-level architecture summary
    async fn generate_repository_summary(
        &self,
        context: &RepositoryContext,
    ) -> Result<String>;
    
    /// Generate module/component-level summary
    async fn generate_component_summary(
        &self,
        context: &ComponentContext,
    ) -> Result<String>;
    
    /// Generate entity-level summary (class, function, etc.)
    async fn generate_entity_summary(
        &self,
        context: &EntityContext,
    ) -> Result<String>;
    
    /// Generate pattern analysis summary (cross-cutting concerns)
    async fn analyze_pattern(
        &self,
        pattern: &str,
        entities: &[CodeEntity],
    ) -> Result<String>;
    
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
```

---

## Data Types

### Context Types

Input context for summary generation.

```rust
/// Repository-level context
pub struct RepositoryContext {
    pub name: String,
    pub languages: Vec<String>,
    pub top_modules: Vec<String>,
    pub file_count: usize,
    pub entity_count: usize,
    pub primary_patterns: Vec<String>,  // e.g., ["CLI", "async", "database"]
}

/// Component/module-level context
pub struct ComponentContext {
    pub name: String,
    pub component_type: ComponentType,
    pub file_count: usize,
    pub dependencies: Vec<String>,      // Names of dependencies
    pub exported_entities: Vec<String>, // Public API surface
    pub primary_patterns: Vec<String>,
}

/// Entity-level context
pub struct EntityContext {
    pub name: String,
    pub entity_type: EntityType,
    pub file_path: String,
    pub code_snippet: String,           // Up to 50 lines
    pub signature: Option<String>,      // For functions/methods
    pub dependencies: Vec<String>,      // Direct dependencies
    pub visibility: Visibility,
}
```

---

### Request Types

```rust
pub enum SummaryRequest {
    Repository(RepositoryContext),
    Component(ComponentContext),
    Entity(EntityContext),
}

pub struct SummaryResult {
    pub request: SummaryRequest,
    pub summary: Option<String>,
    pub error: Option<LlmError>,
    pub generation_time: Duration,
    pub token_count: Option<usize>,
}
```

---

### Service Health

```rust
pub struct ServiceHealth {
    pub is_available: bool,
    pub ollama_version: Option<String>,
    pub loaded_models: Vec<String>,
    pub api_endpoint: String,
    pub response_time: Duration,
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Ollama service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Generation timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid response from LLM: {0}")]
    InvalidResponse(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Context too large: {current} tokens, max {max}")]
    ContextTooLarge { current: usize, max: usize },
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

impl LlmError {
    pub fn to_user_message(&self) -> String {
        match self {
            Self::ServiceUnavailable(_) => 
                "Ollama service is not running. Start it with: ollama serve\n\
                 Or disable knowledge graph: cudgel index --no-graph".to_string(),
            Self::ModelNotFound(model) => 
                format!("Model '{}' not found. Download with: ollama pull {}", model, model),
            Self::Timeout(duration) => 
                format!("Summary generation timed out after {:?}. \n\
                        Try: (1) Use smaller code context, (2) Increase timeout, (3) Use faster model", duration),
            Self::InvalidResponse(msg) => 
                format!("LLM returned invalid response: {}. Check Ollama logs for details.", msg),
            Self::RateLimitExceeded => 
                "Too many concurrent requests to Ollama. Reduce concurrency or wait.".to_string(),
            Self::ContextTooLarge { current, max } => 
                format!("Code context too large ({} tokens, max {}). Reduce code snippet size.", current, max),
            Self::HttpError(e) => 
                format!("Network error communicating with Ollama: {}", e),
        }
    }
}
```

---

## Prompt Templates

### Repository Summary Template

```rust
pub const REPOSITORY_PROMPT: &str = r#"
Analyze this code repository and provide a concise architectural overview.

Repository: {name}
Languages: {languages}
File count: {file_count}
Main modules: {modules}
Key patterns: {patterns}

Provide:
1. Overall architecture pattern (e.g., layered, microservices, monolithic) - 1 sentence
2. Primary components and their purposes - 2-3 sentences
3. Key technologies and frameworks detected - 1 sentence

Keep response factual and under 200 words. Focus on what the codebase does, not implementation details.
"#;
```

---

### Component Summary Template

```rust
pub const COMPONENT_PROMPT: &str = r#"
Summarize the purpose and responsibilities of this code module.

Module: {name}
Type: {component_type}
Files: {file_count}
Dependencies: {dependencies}
Exported API: {exports}

Provide:
1. Primary responsibility - 1 sentence
2. Key functionality - 2-3 bullet points
3. How it fits in the overall architecture - 1 sentence

Keep response under 150 words. Be specific about what this module does.
"#;
```

---

### Entity Summary Template

```rust
pub const ENTITY_PROMPT: &str = r#"
Explain what this {entity_type} does and its purpose.

Name: {name}
File: {file_path}
Visibility: {visibility}

Code:
```
{code_snippet}
```

Provide:
1. What it does - 1 sentence
2. Key responsibilities - 2-3 points
3. Notable patterns or techniques used - 1 sentence

Keep response under 100 words. Be specific and factual.
"#;
```

---

### Pattern Analysis Template

```rust
pub const PATTERN_ANALYSIS_PROMPT: &str = r#"
Analyze how "{pattern}" is implemented across this codebase.

Related entities found: {entity_count}

Sample entities:
{entity_list}

Provide:
1. Overall pattern/approach used - 1-2 sentences
2. Key components involved - 2-3 bullet points
3. Consistency assessment - 1 sentence

Keep response under 150 words. Focus on architectural patterns, not code details.
"#;
```

---

## Implementation Notes

### Generation Parameters

**Recommended defaults**:
```rust
pub struct GenerationConfig {
    pub model: String,              // "llama3.2:3b" (default)
    pub temperature: f32,           // 0.3 (low for factual summaries)
    pub top_p: f32,                 // 0.9
    pub top_k: usize,               // 40
    pub max_tokens: usize,          // 500 (prevent overly long summaries)
    pub timeout: Duration,          // 30 seconds
    pub retry_attempts: usize,      // 2
    pub retry_delay: Duration,      // 5 seconds
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2:3b".to_string(),
            temperature: 0.3,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 500,
            timeout: Duration::from_secs(30),
            retry_attempts: 2,
            retry_delay: Duration::from_secs(5),
        }
    }
}
```

---

### Rate Limiting

To avoid overwhelming Ollama:

```rust
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl RateLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }
    
    pub async fn acquire(&self) -> SemaphorePermit {
        self.semaphore.acquire().await.unwrap()
    }
}
```

**Default limits**:
- Max concurrent requests: 3
- Delay between batches: 100ms

---

### Context Window Management

llama3.2 supports 128k tokens, but practical limits:

```rust
pub const MAX_CODE_SNIPPET_LINES: usize = 50;
pub const MAX_ENTITY_LIST_SIZE: usize = 20;
pub const MAX_MODULE_LIST_SIZE: usize = 30;

pub fn truncate_code_snippet(code: &str) -> String {
    code.lines()
        .take(MAX_CODE_SNIPPET_LINES)
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

### Retry Logic

```rust
pub async fn generate_with_retry(
    client: &LlmClient,
    request: SummaryRequest,
    config: &GenerationConfig,
) -> Result<String> {
    let mut attempts = 0;
    
    loop {
        match execute_generation(client, &request).await {
            Ok(summary) => return Ok(summary),
            Err(e) if attempts < config.retry_attempts => {
                attempts += 1;
                tracing::warn!("Generation failed (attempt {}): {}. Retrying...", attempts, e);
                tokio::time::sleep(config.retry_delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

### Graceful Degradation

When Ollama is unavailable:

```rust
pub async fn generate_repository_summary_safe(
    client: &LlmClient,
    context: &RepositoryContext,
) -> Result<Option<String>> {
    match client.health_check().await {
        Ok(health) if health.is_available => {
            client.generate_repository_summary(context)
                .await
                .map(Some)
        }
        _ => {
            tracing::warn!("Ollama unavailable, skipping summary generation");
            Ok(None)
        }
    }
}
```

---

## Performance Requirements

| Operation | Target Latency | Notes |
|-----------|----------------|-------|
| `generate_repository_summary` | <5s | Use llama3.2:3b |
| `generate_component_summary` | <3s | Smaller context |
| `generate_entity_summary` | <2s | Minimal context |
| `generate_summaries_batch` (10 items) | <15s | 3 concurrent |
| `health_check` | <500ms | Simple ping |

---

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_generate_repository_summary() {
        let client = setup_test_llm_client().await;
        
        let context = RepositoryContext {
            name: "cudgel".to_string(),
            languages: vec!["Rust".to_string()],
            top_modules: vec!["parser".to_string(), "indexer".to_string()],
            file_count: 50,
            entity_count: 500,
            primary_patterns: vec!["CLI".to_string(), "async".to_string()],
        };
        
        let summary = client.generate_repository_summary(&context).await.unwrap();
        
        assert!(!summary.is_empty());
        assert!(summary.len() < 2000, "Summary too long");
        assert!(summary.to_lowercase().contains("rust") || summary.to_lowercase().contains("cli"));
    }
    
    #[tokio::test]
    async fn test_health_check_available() {
        let client = setup_test_llm_client().await;
        
        let health = client.health_check().await.unwrap();
        
        assert!(health.is_available);
        assert!(!health.loaded_models.is_empty());
    }
    
    #[tokio::test]
    async fn test_health_check_unavailable() {
        let client = LlmClient::new("http://invalid:9999").unwrap();
        
        let result = client.health_check().await;
        
        assert!(result.is_err());
        match result {
            Err(LlmError::ServiceUnavailable(_)) => {}
            _ => panic!("Expected ServiceUnavailable error"),
        }
    }
    
    #[tokio::test]
    async fn test_timeout() {
        let mut client = setup_test_llm_client().await;
        client.set_timeout(Duration::from_millis(1)); // Very short timeout
        
        let context = RepositoryContext { /* ... */ };
        let result = client.generate_repository_summary(&context).await;
        
        assert!(result.is_err());
        match result {
            Err(LlmError::Timeout(_)) => {}
            _ => panic!("Expected Timeout error"),
        }
    }
    
    #[tokio::test]
    async fn test_batch_generation_with_rate_limit() {
        let client = setup_test_llm_client().await;
        
        let requests: Vec<SummaryRequest> = (0..10)
            .map(|i| SummaryRequest::Component(make_test_component_context(i)))
            .collect();
        
        let start = std::time::Instant::now();
        let results = client.generate_summaries_batch(requests, 3).await.unwrap();
        let duration = start.elapsed();
        
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.summary.is_some()));
        
        // With concurrency=3, should take ~3x longer than 1 concurrent
        // but much less than 10x (sequential)
        assert!(duration.as_secs() < 30, "Batch too slow: {:?}", duration);
    }
}
```

---

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires Ollama running
async fn test_full_summary_generation_workflow() {
    let client = setup_real_llm_client().await;
    
    // Test with real Ollama service
    let health = client.health_check().await.unwrap();
    assert!(health.is_available);
    
    // Generate repository summary
    let repo_context = RepositoryContext {
        name: "test-repo".to_string(),
        languages: vec!["Rust".to_string()],
        top_modules: vec!["parser".to_string()],
        file_count: 10,
        entity_count: 100,
        primary_patterns: vec!["CLI".to_string()],
    };
    
    let repo_summary = client.generate_repository_summary(&repo_context).await.unwrap();
    println!("Repository summary:\n{}", repo_summary);
    assert!(!repo_summary.is_empty());
    
    // Generate component summary
    let component_context = ComponentContext {
        name: "parser".to_string(),
        component_type: ComponentType::Module,
        file_count: 3,
        dependencies: vec!["tree-sitter".to_string()],
        exported_entities: vec!["parse".to_string(), "Parser".to_string()],
        primary_patterns: vec!["AST".to_string()],
    };
    
    let component_summary = client.generate_component_summary(&component_context).await.unwrap();
    println!("Component summary:\n{}", component_summary);
    assert!(!component_summary.is_empty());
}
```

---

## Contract Validation

### Pre-conditions

- Ollama service is running (for generation operations)
- Model is downloaded (`ollama pull llama3.2:3b`)
- Context data is valid and complete

### Post-conditions

- Generated summaries are non-empty strings
- Summaries are under max token limit
- Error messages provide actionable guidance
- Health checks accurately reflect service state

### Invariants

- Temperature is in range [0.0, 2.0]
- Timeout is positive duration
- Max tokens is positive integer
- Retry attempts is non-negative
- Concurrency limit is positive
