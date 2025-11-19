# Fallback Tokenization Quickstart

**Feature**: 001-fallback-tokenization  
**Date**: 2025-11-19  
**Purpose**: Quick reference for developers implementing and users using fallback tokenization

---

## For Users: Using Fallback Tokenization

### Scenario 1: Restricted Corporate Environment (No Internet)

**Problem**: Cannot download ONNX models from HuggingFace due to firewall/security policies.

**Solution**:
```bash
# Set environment variable to use fallback strategy
export CUDGEL_TOKENIZER_STRATEGY=fallback

# Index your codebase
cudgel index /path/to/codebase

# Search works immediately (no model download needed)
cudgel query "find authentication functions"
```

**What Changes**:
- ✅ No external downloads required
- ✅ Fast initialization (~2 seconds vs. 10-15 seconds)
- ⚠️  Reduced search quality (30-50% degradation for semantic queries)
- ✅ Syntactic searches still work well (70-85% quality)

---

### Scenario 2: Default Setup (ONNX with Best Quality)

**Problem**: None - you have internet access and want best search quality.

**Solution**:
```bash
# Download ONNX models (one-time setup)
cudgel deps

# No environment variable needed - ONNX is default
cudgel index /path/to/codebase
cudgel query "find sorting algorithms"
```

**What You Get**:
- ✅ Best semantic search quality (transformer-based embeddings)
- ⚠️  Requires 90MB model download
- ⚠️  Slower initialization (10-15 seconds)

---

### Scenario 3: Switch Between Strategies

**Problem**: Want to test both strategies or switch based on environment.

**Solution**:
```bash
# Use ONNX on your development machine
unset CUDGEL_TOKENIZER_STRATEGY  # or set to "onnx"
cudgel index ~/projects/myapp

# Use fallback on restricted CI/CD environment
export CUDGEL_TOKENIZER_STRATEGY=fallback
cudgel index /workspace/myapp

# Note: Re-indexing required when switching strategies
# (embeddings are not compatible between strategies)
```

---

### Configuration Check

**Verify Active Strategy**:
```bash
# Run indexing with verbose logging
CUDGEL_DEBUG=1 cudgel index /path/to/codebase

# Look for log line:
# "Initialized embedding generator with 'fallback' strategy"
```

---

### Troubleshooting

**Error: "Invalid tokenization strategy 'xyz'"**
```
Invalid tokenization strategy 'xyz'. Valid options: 'onnx', 'fallback'.

Set via environment variable:
  export CUDGEL_TOKENIZER_STRATEGY=fallback
```

**Fix**: Use one of the valid values (`onnx` or `fallback`).

---

**Error: "ONNX model not found"**
```
ONNX model not found at /path/to/models/model.onnx.

Options:
  1. Download ONNX models: cudgel deps
  2. Use fallback strategy: export CUDGEL_TOKENIZER_STRATEGY=fallback
```

**Fix**: Either download models OR switch to fallback strategy.

---

## For Developers: Implementation Guide

### Phase 1: Test-Driven Development (TDD)

#### Step 1: Write Failing Tests

Create `tests/embeddings_fallback_tests.rs`:

```rust
use cudgel::{Config, embeddings::FallbackTokenizer, TokenizerStrategy};

#[test]
fn test_fallback_produces_384_dimensions() {
    let config = Config::local().unwrap();
    let tokenizer = FallbackTokenizer::initialize(&config).unwrap();
    
    let embedding = tokenizer.encode("fn test() -> i32 { 42 }").unwrap();
    
    // MUST FAIL initially (FallbackTokenizer doesn't exist yet)
    assert_eq!(embedding.len(), 384);
}

#[test]
fn test_fallback_is_deterministic() {
    let config = Config::local().unwrap();
    let tokenizer = FallbackTokenizer::initialize(&config).unwrap();
    
    let text = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let emb1 = tokenizer.encode(text).unwrap();
    let emb2 = tokenizer.encode(text).unwrap();
    
    // MUST FAIL initially
    assert_eq!(emb1, emb2);
}

#[test]
fn test_fallback_is_normalized() {
    let config = Config::local().unwrap();
    let tokenizer = FallbackTokenizer::initialize(&config).unwrap();
    
    let embedding = tokenizer.encode("test code").unwrap();
    let norm: f32 = embedding.iter().map(|x| x*x).sum::<f32>().sqrt();
    
    // MUST FAIL initially
    assert!((norm - 1.0).abs() < 0.01, "Norm: {}", norm);
}
```

**Run Tests** (expect failures):
```bash
cargo test test_fallback
# Expected: compilation errors (FallbackTokenizer doesn't exist)
```

---

#### Step 2: Define Trait (Make Tests Compile)

Create `src/embeddings/mod.rs`:

```rust
use crate::{Config, Result};

/// Strategy for tokenizing text and generating embeddings
pub trait TokenizerStrategy: Send + Sync {
    fn initialize(config: &Config) -> Result<Self> where Self: Sized;
    fn encode(&self, text: &str) -> Result<Vec<f32>>;
    fn validate(&self) -> Result<()>;
    fn name(&self) -> &'static str;
}

pub enum EmbedderBackend {
    Onnx(OnnxTokenizer),
    Fallback(FallbackTokenizer),
}

// Re-export for tests
pub use self::fallback::FallbackTokenizer;
pub use self::onnx::OnnxTokenizer;

mod onnx;
mod fallback;
```

**Run Tests** (still fail, but now compile):
```bash
cargo test test_fallback
# Expected: runtime failures (FallbackTokenizer methods panic/unimplemented)
```

---

#### Step 3: Implement Minimum to Pass Tests

Create `src/embeddings/fallback.rs`:

```rust
use crate::{Config, Result, Error};
use crate::embeddings::TokenizerStrategy;
use ndarray::Array2;
use rand::{SeedableRng, Rng};
use rand_chacha::ChaCha8Rng;

pub struct FallbackTokenizer {
    projection_matrix: Array2<f32>,
    hash_dimension: usize,
}

impl TokenizerStrategy for FallbackTokenizer {
    fn initialize(_config: &Config) -> Result<Self> {
        const HASH_DIM: usize = 8192;
        const EMBED_DIM: usize = 384;
        const SEED: u64 = 42;
        
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);
        let scaling = (1.0 / (HASH_DIM as f32).sqrt());
        
        let projection_matrix = Array2::from_shape_fn((EMBED_DIM, HASH_DIM), |_| {
            (rng.gen::<f32>() * 2.0 - 1.0) * scaling
        });
        
        Ok(Self {
            projection_matrix,
            hash_dimension: HASH_DIM,
        })
    }
    
    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // 1. Tokenize
        let tokens = self.tokenize_code(text);
        
        // 2. Hash to sparse vector
        let sparse = self.create_feature_vector(&tokens);
        
        // 3. Project to 384D
        let embedding = self.project_to_embedding(&sparse);
        
        // 4. Normalize
        let mut normalized = embedding;
        self.normalize(&mut normalized);
        
        Ok(normalized)
    }
    
    fn validate(&self) -> Result<()> {
        let shape = self.projection_matrix.shape();
        if shape[0] != 384 || shape[1] != self.hash_dimension {
            return Err(Error::Embedding(format!(
                "Invalid projection matrix shape: {:?}",
                shape
            )));
        }
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "fallback"
    }
}

// Helper methods (implement based on research.md)
impl FallbackTokenizer {
    fn tokenize_code(&self, text: &str) -> Vec<String> {
        // Split on whitespace, handle camelCase/snake_case
        text.split_whitespace()
            .flat_map(|word| self.split_identifier(word))
            .map(|s| s.to_lowercase())
            .collect()
    }
    
    fn split_identifier(&self, s: &str) -> Vec<String> {
        // Handle camelCase: getUserName → [get, User, Name]
        // Handle snake_case: get_user_name → [get, user, name]
        // (Simplified implementation for MVP)
        vec![s.to_string()]
    }
    
    fn create_feature_vector(&self, tokens: &[String]) -> Vec<f32> {
        use xxhash_rust::xxh3::xxh3_64;
        
        let mut sparse = vec![0.0; self.hash_dimension];
        
        for token in tokens {
            let idx = (xxh3_64(token.as_bytes()) % self.hash_dimension as u64) as usize;
            let sign = if (xxh3_64(format!("sign_{}", token).as_bytes()) & 1) == 0 {
                1.0
            } else {
                -1.0
            };
            sparse[idx] += sign;
        }
        
        sparse
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

**Run Tests** (should pass):
```bash
cargo test test_fallback
# Expected: GREEN (all tests pass)
```

---

### Phase 2: Refactor Existing ONNX Code

#### Step 1: Extract ONNX to Separate Module

Move existing `EmbeddingGenerator` code to `src/embeddings/onnx.rs`:

```rust
use crate::{Config, Result, Error};
use crate::embeddings::TokenizerStrategy;
use ort::session::Session;
use tokenizers::Tokenizer;
use std::sync::Mutex;

pub struct OnnxTokenizer {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl TokenizerStrategy for OnnxTokenizer {
    fn initialize(config: &Config) -> Result<Self> {
        // [Copy existing initialization code from embeddings.rs]
        // - Load ONNX model from config.embedding.model_path
        // - Load tokenizer from config.embedding.model_path
        // ...
    }
    
    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // [Copy existing encoding pipeline from embeddings.rs]
        // - Tokenize
        // - ONNX inference
        // - Mean pooling
        // - L2 normalize
        // ...
    }
    
    fn validate(&self) -> Result<()> {
        // Check ONNX session is locked successfully
        let _session = self.session.lock()
            .map_err(|e| Error::Embedding(format!("Session lock failed: {}", e)))?;
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "onnx"
    }
}
```

**Test Refactor** (ensure ONNX still works):
```bash
cargo test test_embedding  # Existing ONNX tests
# Expected: GREEN (no regressions)
```

---

#### Step 2: Update EmbeddingGenerator

Modify `src/embeddings.rs`:

```rust
use crate::{Config, Result, Error};
use crate::embeddings::{EmbedderBackend, OnnxTokenizer, FallbackTokenizer};
use std::sync::Arc;

pub struct EmbeddingGenerator {
    config: Arc<Config>,
    backend: EmbedderBackend,
}

impl EmbeddingGenerator {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let backend = match config.embedding.strategy.as_str() {
            "onnx" => {
                EmbedderBackend::Onnx(OnnxTokenizer::initialize(&config)?)
            }
            "fallback" => {
                EmbedderBackend::Fallback(FallbackTokenizer::initialize(&config)?)
            }
            s => return Err(Error::InvalidTokenizerStrategy(s.to_string())),
        };
        
        tracing::info!("Initialized embedding generator with '{}' strategy", 
                       backend.name());
        
        Ok(Self { config, backend })
    }
    
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        self.backend.encode(text)
    }
    
    // ... existing encode_symbol, encode_code, encode_query methods ...
    // (no changes needed - they use self.encode internally)
}
```

---

### Phase 3: Configuration Integration

#### Update Config

Modify `src/config.rs`:

```rust
pub struct EmbeddingConfig {
    pub model_path: PathBuf,
    pub dimension: usize,
    pub strategy: String,  // NEW
}

impl Config {
    pub fn local() -> Result<Self> {
        // ... existing code ...
        
        embedding: EmbeddingConfig {
            model_path: xdg_data_home().join("cudgel/models/all-MiniLM-L6-v2"),
            dimension: 384,
            strategy: std::env::var("CUDGEL_TOKENIZER_STRATEGY")
                .unwrap_or_else(|_| "onnx".to_string())
                .to_lowercase(),  // NEW
        },
        
        // ... rest of config ...
    }
}
```

#### Add Validation

```rust
impl Config {
    pub fn validate(&self) -> Result<()> {
        // ... existing validations ...
        
        // Validate tokenization strategy
        match self.embedding.strategy.as_str() {
            "onnx" | "fallback" => Ok(()),
            invalid => Err(Error::Config(format!(
                "Invalid tokenization strategy '{}'. Valid options: 'onnx', 'fallback'.\n\
                 \n\
                 Set via environment variable:\n\
                 export CUDGEL_TOKENIZER_STRATEGY=fallback\n\
                 \n\
                 Strategy details:\n\
                 • 'onnx' (default): Best quality, requires model download\n\
                 • 'fallback': Offline mode, no downloads, reduced quality",
                invalid
            ))),
        }
    }
}
```

---

### Phase 4: Integration Tests

Create `tests/integration_tests.rs` additions:

```rust
#[test]
fn test_strategy_switching_via_env_var() {
    // Test fallback strategy
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "fallback");
    let config = Config::local().unwrap();
    let generator = EmbeddingGenerator::new(Arc::new(config)).unwrap();
    
    let embedding = generator.encode("fn test() {}").unwrap();
    assert_eq!(embedding.len(), 384);
    
    // Test ONNX strategy (if models available)
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "onnx");
    let config = Config::local().unwrap();
    
    if let Ok(generator) = EmbeddingGenerator::new(Arc::new(config)) {
        let embedding = generator.encode("fn test() {}").unwrap();
        assert_eq!(embedding.len(), 384);
    }
}

#[test]
fn test_invalid_strategy_rejected() {
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "invalid");
    let result = Config::local();
    assert!(result.is_err());
}

#[test]
fn test_fallback_quality_baseline() {
    std::env::set_var("CUDGEL_TOKENIZER_STRATEGY", "fallback");
    let config = Config::local().unwrap();
    let generator = EmbeddingGenerator::new(Arc::new(config)).unwrap();
    
    // Similar code should have high cosine similarity
    let code1 = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let code2 = "fn add(x: i32, y: i32) -> i32 { x + y }";
    
    let emb1 = generator.encode(code1).unwrap();
    let emb2 = generator.encode(code2).unwrap();
    
    let similarity = cosine_similarity(&emb1, &emb2);
    assert!(similarity > 0.7, "Syntactic similarity should be high: {}", similarity);
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    // Vectors are already normalized, so dot product = cosine similarity
    dot
}
```

---

## File Modification Checklist

### New Files
- [x] `src/embeddings/mod.rs` - Trait, enum, factory
- [x] `src/embeddings/fallback.rs` - Fallback implementation
- [x] `src/embeddings/onnx.rs` - Extracted ONNX implementation
- [x] `tests/embeddings_fallback_tests.rs` - Fallback unit tests

### Modified Files
- [x] `src/embeddings.rs` - Refactor to use backend abstraction
- [x] `src/config.rs` - Add `strategy` field to `EmbeddingConfig`
- [x] `src/error.rs` - Add `InvalidTokenizerStrategy` variant
- [x] `src/lib.rs` - Update module exports
- [x] `tests/integration_tests.rs` - Add strategy switching tests
- [x] `Cargo.toml` - Add dependencies (`rand_chacha`, `unicode-segmentation`, `xxhash-rust`)

---

## Development Commands

### Build
```bash
cargo build --release
# or
task build
```

### Test All
```bash
cargo test
# or
task test
```

### Test Specific Module
```bash
cargo test --test embeddings_fallback_tests
cargo test test_fallback
```

### Test with Specific Strategy
```bash
CUDGEL_TOKENIZER_STRATEGY=fallback cargo test
CUDGEL_TOKENIZER_STRATEGY=onnx cargo test
```

### Lint (Zero Warnings Policy)
```bash
cargo clippy --all-targets -- -D warnings
# or
task clippy
```

### Format
```bash
cargo fmt
# or
task fmt
```

---

## Quality Validation

### Benchmark Queries (Manual Testing)

After implementation, test with cudgel's own codebase:

```bash
# Index cudgel with fallback
export CUDGEL_TOKENIZER_STRATEGY=fallback
cudgel index .

# Test queries
cudgel query "embedding generation"
cudgel query "database connection"
cudgel query "error handling"
cudgel query "tokenization"
cudgel query "vector search"

# Compare with ONNX
export CUDGEL_TOKENIZER_STRATEGY=onnx
cudgel deps  # Download models
cudgel index .

# Run same queries, compare results
```

**Acceptance Criteria**:
- Fallback finds at least 50% of relevant results found by ONNX
- Top-3 results include at least 1 relevant result
- No crashes or errors during indexing or querying

---

## Performance Targets

### Initialization Time
```bash
time cudgel index /path/to/small/repo
# Fallback target: <5 seconds total (including DB connection)
# ONNX baseline: 10-15 seconds
```

### Indexing Throughput
```bash
# Should not degrade from current baseline
# Target: 5 minutes for 10k files
```

### Memory Usage
```bash
# Monitor with:
/usr/bin/time -l cudgel index /path/to/repo

# Fallback target: <500 MB RSS
# (Should be LESS than ONNX due to no model in memory)
```

---

## Troubleshooting Implementation Issues

### Compilation Errors

**"cannot find type `TokenizerStrategy` in this scope"**
- Ensure `src/embeddings/mod.rs` exports the trait
- Check `src/lib.rs` has `pub mod embeddings;`

**"trait objects without explicit `dyn` are deprecated"**
- Use `Box<dyn TokenizerStrategy>` instead of `Box<TokenizerStrategy>`

### Test Failures

**"assertion failed: embedding.len() == 384"**
- Check `project_to_embedding` matrix dimensions
- Verify projection matrix is 384 rows × 8192 columns

**"assertion failed: (norm - 1.0).abs() < 0.01"**
- Check `normalize` implementation
- Ensure division by norm happens for all elements

### Runtime Errors

**"Invalid projection matrix shape"**
- Check random number generator seeding
- Verify `Array2::from_shape_fn` arguments

**"Hash dimension mismatch"**
- Ensure `HASH_DIM` constant consistent across methods
- Check sparse vector creation uses correct dimension

---

## Success Criteria Verification

| Criterion | Test Method | Target |
|-----------|-------------|--------|
| SC-001: Index in restricted env | Set `CUDGEL_TOKENIZER_STRATEGY=fallback`, run index | ✅ Success |
| SC-002: Init <5 seconds | Time `EmbeddingGenerator::new()` | <5s |
| SC-003: Actionable errors | Test invalid strategy, check error message | Includes syntax + options |
| SC-004: 100% indexing success | Index cudgel codebase with fallback | No failures |
| SC-005: Meaningful results | Manual query validation | 50-70% quality vs. ONNX |

---

## Next Steps

After implementation:
1. ✅ All tests pass (`cargo test`)
2. ✅ Zero clippy warnings (`cargo clippy --all-targets -- -D warnings`)
3. ✅ Code formatted (`cargo fmt`)
4. ✅ Manual quality validation complete
5. ✅ Performance targets met
6. ⏳ Documentation updated (README, AGENTS.md)
7. ⏳ Ready for code review and merge

---

## Additional Resources

- [Research Document](./research.md) - Algorithm selection rationale
- [Data Model](./data-model.md) - Entity definitions
- [Trait Contract](./contracts/tokenizer-strategy.md) - Interface specification
- [Feature Spec](./spec.md) - User requirements and acceptance criteria
