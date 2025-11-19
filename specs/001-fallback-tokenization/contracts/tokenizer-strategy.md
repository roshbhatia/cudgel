# TokenizerStrategy Trait Contract

**Feature**: 001-fallback-tokenization  
**Date**: 2025-11-19  
**Purpose**: Define interface contract for all tokenization strategy implementations

---

## Overview

The `TokenizerStrategy` trait defines the contract that all tokenization implementations must satisfy to be compatible with the `EmbeddingGenerator`. This ensures consistent behavior, thread safety, and output format across different strategies.

---

## Trait Definition

```rust
use crate::{Config, Result};

/// Strategy for tokenizing text and generating embeddings
///
/// All implementations must produce 384-dimensional L2-normalized vectors
/// and be thread-safe (Send + Sync) for concurrent indexing.
pub trait TokenizerStrategy: Send + Sync {
    /// Initialize the strategy with given configuration
    ///
    /// # Arguments
    /// * `config` - Application configuration containing strategy-specific settings
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully initialized strategy
    /// * `Err(Error)` - Initialization failed (missing resources, invalid config, etc.)
    ///
    /// # Errors
    /// Must return error with actionable troubleshooting steps if:
    /// - Required model files are missing (ONNX strategy)
    /// - Configuration is invalid
    /// - Resource allocation fails (memory, file handles)
    fn initialize(config: &Config) -> Result<Self>
    where
        Self: Sized;

    /// Encode text into a 384-dimensional embedding vector
    ///
    /// # Arguments
    /// * `text` - Input text to encode (code snippet, query, symbol name)
    ///
    /// # Returns
    /// * `Ok(Vec<f32>)` - 384-dimensional L2-normalized embedding vector
    /// * `Err(Error)` - Encoding failed (invalid input, tokenization error)
    ///
    /// # Guarantees
    /// - Output vector length is ALWAYS 384
    /// - Output vector is L2 normalized (unit length)
    /// - Same input produces identical output (deterministic)
    ///
    /// # Errors
    /// May return error if:
    /// - Text is empty or invalid UTF-8
    /// - Internal tokenization fails
    /// - Memory allocation fails
    fn encode(&self, text: &str) -> Result<Vec<f32>>;

    /// Validate that strategy is ready to encode
    ///
    /// # Returns
    /// * `Ok(())` - Strategy is valid and ready
    /// * `Err(Error)` - Strategy is in invalid state
    ///
    /// # Purpose
    /// Called after initialization to verify strategy can encode.
    /// Useful for detecting corrupted state or missing resources.
    fn validate(&self) -> Result<()>;

    /// Return human-readable strategy name for logging
    ///
    /// # Returns
    /// Static string identifying the strategy (e.g., "onnx", "fallback")
    ///
    /// # Purpose
    /// Used for diagnostic logging and error messages.
    fn name(&self) -> &'static str;
}
```

---

## Invariants

All implementations MUST guarantee these invariants:

### 1. Fixed Dimension

**Requirement**: `encode()` MUST always return a `Vec<f32>` with exactly 384 elements.

**Test**:
```rust
#[test]
fn test_dimension_invariant() {
    let strategy = TestStrategy::initialize(&config()).unwrap();
    let embedding = strategy.encode("test input").unwrap();
    assert_eq!(embedding.len(), 384, "Must output exactly 384 dimensions");
}
```

**Rationale**: Ensures compatibility with existing pgvector schema (`vector(384)`).

---

### 2. L2 Normalization

**Requirement**: Output vectors MUST be L2-normalized (unit length: norm = 1.0 ± 0.01).

**Test**:
```rust
#[test]
fn test_normalization_invariant() {
    let strategy = TestStrategy::initialize(&config()).unwrap();
    let embedding = strategy.encode("test input").unwrap();
    
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01, "Must be L2 normalized (norm={})", norm);
}
```

**Rationale**: Ensures cosine similarity comparisons are meaningful (pgvector's `<=>` operator).

---

### 3. Determinism

**Requirement**: Same input text MUST produce identical output embeddings within a strategy.

**Test**:
```rust
#[test]
fn test_determinism_invariant() {
    let strategy = TestStrategy::initialize(&config()).unwrap();
    let text = "fn calculate_sum(items: &[i32]) -> i32";
    
    let embedding1 = strategy.encode(text).unwrap();
    let embedding2 = strategy.encode(text).unwrap();
    
    assert_eq!(embedding1, embedding2, "Must be deterministic");
}
```

**Rationale**: Ensures reproducible search results and prevents database inconsistencies.

---

### 4. Thread Safety

**Requirement**: All methods must be safe to call from multiple threads (trait bound: `Send + Sync`).

**Test**:
```rust
#[test]
fn test_thread_safety_invariant() {
    use std::sync::Arc;
    use std::thread;
    
    let strategy = Arc::new(TestStrategy::initialize(&config()).unwrap());
    let mut handles = vec![];
    
    for i in 0..10 {
        let s = Arc::clone(&strategy);
        handles.push(thread::spawn(move || {
            s.encode(&format!("test input {}", i)).unwrap()
        }));
    }
    
    for handle in handles {
        handle.join().unwrap(); // Must not panic
    }
}
```

**Rationale**: Enables concurrent indexing of multiple files without race conditions.

---

### 5. Error Transparency

**Requirement**: Errors MUST include actionable troubleshooting information.

**Example Error Messages**:
```rust
// Good: Actionable error
Err(Error::Embedding(
    "ONNX model not found at /path/to/model.onnx. \
     Run 'cudgel deps' to download models, or use fallback: \
     export CUDGEL_TOKENIZER_STRATEGY=fallback"
))

// Bad: Opaque error
Err(Error::Embedding("Model not found"))
```

**Rationale**: Reduces user frustration by providing clear remediation steps.

---

## Implementation Requirements

### OnnxTokenizer (Refactored from existing code)

#### Struct Definition

```rust
pub(crate) struct OnnxTokenizer {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    config: Arc<Config>,
}
```

#### Implementation

```rust
impl TokenizerStrategy for OnnxTokenizer {
    fn initialize(config: &Config) -> Result<Self> {
        let model_path = &config.embedding.model_path;
        
        // Validate model files exist
        let model_file = model_path.join("model.onnx");
        let tokenizer_file = model_path.join("tokenizer.json");
        
        if !model_file.exists() {
            return Err(Error::Embedding(format!(
                "ONNX model not found at {:?}. \
                 Run 'cudgel deps' to download models, or use fallback: \
                 export CUDGEL_TOKENIZER_STRATEGY=fallback",
                model_file
            )));
        }
        
        // Initialize ONNX Runtime
        ort::init().with_name("cudgel").commit()
            .map_err(|e| Error::Embedding(format!("ONNX init failed: {}", e)))?;
        
        // Load ONNX model
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_file)?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| Error::Embedding(format!("Tokenizer load failed: {}", e)))?;
        
        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            config: Arc::new(config.clone()),
        })
    }
    
    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // [Existing ONNX encoding pipeline - no changes]
        // 1. Tokenize
        // 2. ONNX inference
        // 3. Mean pooling
        // 4. L2 normalize
        // ...
    }
    
    fn validate(&self) -> Result<()> {
        // Check ONNX session is initialized
        let _session = self.session.lock()
            .map_err(|e| Error::Embedding(format!("Session lock failed: {}", e)))?;
        
        // Verify model files still accessible
        let model_path = &self.config.embedding.model_path;
        if !model_path.join("model.onnx").exists() {
            return Err(Error::Embedding("ONNX model file no longer accessible".into()));
        }
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "onnx"
    }
}
```

---

### FallbackTokenizer (New implementation)

#### Struct Definition

```rust
use ndarray::Array2;
use rand::{SeedableRng, Rng};
use rand_chacha::ChaCha8Rng;
use xxhash_rust::xxh3::xxh3_64;

pub(crate) struct FallbackTokenizer {
    projection_matrix: Array2<f32>,
    hash_dimension: usize,
    seed: u64,
}
```

#### Implementation

```rust
impl TokenizerStrategy for FallbackTokenizer {
    fn initialize(config: &Config) -> Result<Self> {
        const HASH_DIM: usize = 8192;
        const EMBED_DIM: usize = 384;
        const SEED: u64 = 42;
        
        tracing::info!("Initializing fallback tokenizer ({}D → {}D)", HASH_DIM, EMBED_DIM);
        
        // Generate random projection matrix
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);
        let scaling = (1.0 / (HASH_DIM as f32).sqrt());
        
        let projection_matrix = Array2::from_shape_fn((EMBED_DIM, HASH_DIM), |_| {
            (rng.gen::<f32>() * 2.0 - 1.0) * scaling
        });
        
        Ok(Self {
            projection_matrix,
            hash_dimension: HASH_DIM,
            seed: SEED,
        })
    }
    
    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // 1. Tokenize code
        let tokens = self.tokenize_code(text);
        
        // 2. Feature hashing
        let sparse_vector = self.create_feature_vector(&tokens);
        
        // 3. Random projection
        let embedding = self.project_to_embedding(&sparse_vector);
        
        // 4. L2 normalize
        let mut normalized = embedding;
        self.normalize(&mut normalized);
        
        Ok(normalized)
    }
    
    fn validate(&self) -> Result<()> {
        // Check projection matrix dimensions
        let shape = self.projection_matrix.shape();
        if shape[0] != 384 || shape[1] != self.hash_dimension {
            return Err(Error::Embedding(format!(
                "Invalid projection matrix shape: {:?} (expected [384, {}])",
                shape, self.hash_dimension
            )));
        }
        
        // Always OK (no external dependencies)
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "fallback"
    }
}

// Helper methods (private)
impl FallbackTokenizer {
    fn tokenize_code(&self, text: &str) -> Vec<String> {
        // Split on whitespace, split camelCase/snake_case, lowercase
        // ...
    }
    
    fn create_feature_vector(&self, tokens: &[String]) -> Vec<f32> {
        let mut sparse = vec![0.0; self.hash_dimension];
        
        for token in tokens {
            let (idx, sign) = self.hash_token(token);
            sparse[idx] += sign;
        }
        
        sparse
    }
    
    fn hash_token(&self, token: &str) -> (usize, f32) {
        let idx = (xxh3_64(token.as_bytes()) % self.hash_dimension as u64) as usize;
        let sign = if (xxh3_64(format!("sign_{}", token).as_bytes()) & 1) == 0 {
            1.0
        } else {
            -1.0
        };
        (idx, sign)
    }
    
    fn project_to_embedding(&self, sparse: &[f32]) -> Vec<f32> {
        use ndarray::Array1;
        let input = Array1::from(sparse.to_vec());
        let output = self.projection_matrix.dot(&input);
        output.to_vec()
    }
    
    fn normalize(&self, vec: &mut [f32]) {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
    }
}
```

---

## Usage Example

### Strategy Selection at Runtime

```rust
use crate::embeddings::{EmbedderBackend, TokenizerStrategy};

// Factory pattern (in EmbeddingGenerator::new)
let backend = match config.embedding.strategy.as_str() {
    "onnx" => {
        EmbedderBackend::Onnx(OnnxTokenizer::initialize(&config)?)
    }
    "fallback" => {
        EmbedderBackend::Fallback(FallbackTokenizer::initialize(&config)?)
    }
    s => return Err(Error::InvalidTokenizerStrategy(s.to_string())),
};

tracing::info!("Initialized tokenizer strategy: {}", backend.name());
```

### Encoding Usage (Same for All Strategies)

```rust
let embedding = backend.encode("fn calculate_total(items: &[Item]) -> f64")?;

// Invariants guaranteed by trait contract
assert_eq!(embedding.len(), 384);

let norm: f32 = embedding.iter().map(|x| x*x).sum::<f32>().sqrt();
assert!((norm - 1.0).abs() < 0.01);
```

---

## Testing Requirements

### Per-Strategy Tests

Each implementation MUST have these unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dimension_correctness() {
        let strategy = TestStrategy::initialize(&test_config()).unwrap();
        let embedding = strategy.encode("test input").unwrap();
        assert_eq!(embedding.len(), 384);
    }
    
    #[test]
    fn test_determinism() {
        let strategy = TestStrategy::initialize(&test_config()).unwrap();
        let text = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let emb1 = strategy.encode(text).unwrap();
        let emb2 = strategy.encode(text).unwrap();
        assert_eq!(emb1, emb2);
    }
    
    #[test]
    fn test_normalization() {
        let strategy = TestStrategy::initialize(&test_config()).unwrap();
        let embedding = strategy.encode("test input").unwrap();
        let norm: f32 = embedding.iter().map(|x| x*x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;
        
        let strategy = Arc::new(TestStrategy::initialize(&test_config()).unwrap());
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let s = Arc::clone(&strategy);
                thread::spawn(move || s.encode(&format!("input {}", i)).unwrap())
            })
            .collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
    }
    
    #[test]
    fn test_validation() {
        let strategy = TestStrategy::initialize(&test_config()).unwrap();
        assert!(strategy.validate().is_ok());
    }
    
    #[test]
    fn test_name() {
        let strategy = TestStrategy::initialize(&test_config()).unwrap();
        assert!(!strategy.name().is_empty());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_strategy_switching_via_env_var() {
    // Test ONNX strategy
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "onnx");
    let config = Config::local().unwrap();
    let backend = EmbedderBackend::from_config(&config).unwrap();
    assert_eq!(backend.name(), "onnx");
    
    // Test fallback strategy
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "fallback");
    let config = Config::local().unwrap();
    let backend = EmbedderBackend::from_config(&config).unwrap();
    assert_eq!(backend.name(), "fallback");
}

#[test]
fn test_invalid_strategy_error() {
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "invalid");
    let config = Config::local();
    assert!(config.is_err());
}
```

---

## Performance Guarantees

### Initialization Time

| Strategy | Target | Expected | Measured |
|----------|--------|----------|----------|
| ONNX | <15 seconds | 10-12s | ✅ Baseline |
| Fallback | <5 seconds | ~2s | ⏳ To verify |

### Encoding Time

| Strategy | Target | Expected |
|----------|--------|----------|
| ONNX | <50ms per text | 20-30ms |
| Fallback | <10ms per text | ~5ms |

### Memory Footprint

| Strategy | Model Files | Runtime Memory |
|----------|-------------|----------------|
| ONNX | ~90 MB | ~200 MB |
| Fallback | 0 MB | ~15 MB |

---

## Error Scenarios

### Strategy Initialization Errors

**ONNX - Model Files Missing**:
```rust
Err(Error::Embedding(
    "ONNX model not found at /path/to/models/model.onnx. \n\
     Options:\n\
     1. Download models: cudgel deps\n\
     2. Use fallback: export CUDGEL_TOKENIZER_STRATEGY=fallback"
))
```

**ONNX - Runtime Initialization Failed**:
```rust
Err(Error::Embedding(
    "ONNX runtime initialization failed: <reason>. \n\
     Ensure ort dependencies are installed. \n\
     Fallback: export CUDGEL_TOKENIZER_STRATEGY=fallback"
))
```

**Fallback - Memory Allocation Failed** (rare):
```rust
Err(Error::Embedding(
    "Failed to allocate projection matrix (12.5 MB). \n\
     Free system memory and retry."
))
```

### Encoding Errors

**Empty Input**:
```rust
// Both strategies should handle gracefully
let embedding = strategy.encode("")?;
// Returns zero vector or minimal embedding
```

**Invalid UTF-8** (unlikely with Rust strings):
```rust
// Rust &str guarantees valid UTF-8, but if somehow invalid:
Err(Error::Embedding("Invalid UTF-8 in input text"))
```

---

## Version Compatibility

### Trait Stability

**Contract Version**: 1.0

**Backward Compatibility Promise**:
- Adding optional methods (with defaults) is allowed
- Changing method signatures is a breaking change
- Removing methods is a breaking change

**Future Extensions** (non-breaking):
```rust
pub trait TokenizerStrategy: Send + Sync {
    // ... existing methods ...
    
    /// Return strategy metadata (optional, default implementation)
    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata::default()
    }
    
    /// Return expected quality score (optional, 0.0-1.0)
    fn quality_estimate(&self) -> f32 {
        1.0 // Default: assume high quality
    }
}
```

---

## Summary

The `TokenizerStrategy` trait provides a stable, extensible interface for embedding generation with:

1. **Strong Guarantees**: Fixed dimensions, normalization, determinism, thread safety
2. **Clear Error Handling**: Actionable troubleshooting information
3. **Performance Targets**: Initialization time, encoding time, memory usage
4. **Comprehensive Testing**: Unit tests for invariants, integration tests for switching
5. **Future Extensibility**: Easy to add new strategies (e.g., "hybrid", "cached")

All implementations must pass the trait compliance test suite before merging.
