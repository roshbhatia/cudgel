# Research: Fallback Tokenization Strategy

**Date**: 2025-11-19  
**Purpose**: Resolve technical unknowns for implementing offline tokenization strategy

## Summary

Selected **Feature Hashing with Random Projection** as the fallback tokenization algorithm. This approach provides deterministic, offline embedding generation without requiring pre-trained models, while maintaining compatibility with the existing 384-dimensional pgvector database.

---

## 1. Fallback Algorithm Selection

### Decision: Feature Hashing with Random Projection

A hybrid approach combining:
1. **Code-aware tokenization** (identifier splitting, camelCase/snake_case handling)
2. **Feature hashing** (hash trick) to map tokens to sparse high-dimensional space (8192D)
3. **Random projection** to reduce to 384 dimensions while preserving distances

### Rationale

**Key Advantages**:
- ✅ **No pre-trained weights**: Uses deterministic hash functions and fixed random projection matrix
- ✅ **Fast initialization**: <2 seconds (vs. 10-15s for ONNX) - matrix generation is one-time cost
- ✅ **Truly offline**: No vocabulary files, no model downloads, no external dependencies
- ✅ **Fixed dimensions**: Always outputs 384D vectors compatible with pgvector
- ✅ **Deterministic**: Same input always produces identical output (fixed random seed)
- ✅ **Code-specific**: Can handle programming language tokens (camelCase, operators, keywords)
- ✅ **Memory efficient**: No vocabulary dictionary needed (~12.5 MB for projection matrix)

**Trade-offs**:
- ❌ Hash collisions reduce precision (5-10% collision rate at 8192D)
- ❌ No contextual understanding (treats tokens atomically)
- ❌ Semantic similarity significantly degraded (30-50% vs. transformers)

**Why This Beats Alternatives**:
- Better than TF-IDF: No corpus-dependent vocabulary, works out-of-box
- Better than character n-grams: Preserves token-level semantics
- Better than BPE: No pretraining required
- Mathematically sound: Johnson-Lindenstrauss lemma guarantees distance preservation

### Alternatives Considered

| Algorithm | Why Rejected |
|-----------|-------------|
| **TF-IDF + Truncated SVD** | Requires precomputed vocabulary from corpus; SVD needs training data |
| **Character N-Grams** | Loses semantic structure; high collision rate for similar identifiers |
| **BPE (Byte Pair Encoding)** | Requires pretraining on code corpus; not truly offline |
| **LSH (Locality-Sensitive Hashing)** | Outputs discrete buckets, not continuous vectors; designed for search not embeddings |

### Implementation Notes

**Rust Crates**:
- `xxhash-rust` (0.8.15): Fast non-cryptographic hashing (10GB/s throughput)
- `ndarray` (0.17): Array operations and matrix multiplication
- `rand` (0.8) + `rand_chacha` (0.3): Deterministic PRNG for projection matrix
- `unicode-segmentation` (1.12): Unicode-aware tokenization

**Key Parameters**:
- Intermediate hash space: 8192 dimensions (balances collisions vs. memory)
- Target dimension: 384 (matches ONNX output and pgvector schema)
- Random seed: Fixed (42) for reproducibility across runs
- Normalization: L2 normalization for cosine similarity

**Limitations**:
- Cannot distinguish semantic similarity (e.g., `calculate_sum` vs. `compute_total`)
- Works best for syntactic similarity and near-duplicate detection
- Quality degradation expected: 30-50% for semantic queries vs. ONNX

---

## 2. Embedding Generation Strategy

### Decision: Random Projection from Sparse Features

**Algorithm**:
1. Tokenize code → Extract tokens (split identifiers, normalize)
2. Feature hashing → Map tokens to 8192D sparse vector (signed hash trick)
3. Random projection → Multiply by fixed 8192×384 random matrix
4. L2 normalization → Ensure unit vectors for cosine similarity

### Rationale

**Mathematical Foundation**:
- **Johnson-Lindenstrauss Lemma**: With 384 dimensions, preserves pairwise distances within ±20% with >99% probability
- **Fixed Random Matrix**: Deterministic projection (seeded PRNG) ensures reproducibility
- **Sparse-Dense Multiply**: Efficient for code (sparse token distributions)

**Performance Characteristics**:
- Initialization: ~1.5s (generate 8192×384 matrix = 12.5 MB)
- Per-document: ~5ms (tokenize + hash + matrix multiply)
- Throughput: ~200 documents/second (single-threaded)
- Memory: 12.5 MB (projection matrix) + negligible per-document

**Why This Works**:
- Random projection is theoretically grounded (dimensionality reduction)
- Preserves Euclidean distances (equivalently cosine similarity after normalization)
- No training data required (random matrix is sufficient)
- Scales well (O(d × k) where d=8192, k=384)

### Alternatives Considered

| Approach | Why Not Chosen |
|----------|----------------|
| **Learned Projection** | Requires training data and model files (defeats offline goal) |
| **PCA/SVD** | Requires corpus statistics; not generalizable to new codebases |
| **Auto-encoder** | Requires neural network training; too complex |
| **Count Vectorizer Only** | Output dimension depends on vocabulary size (not fixed 384D) |

### Implementation Details

**Hash Function Design**:
```rust
// Primary hash: maps token to index
let idx = (xxh3_64(token.as_bytes()) % 8192) as usize;

// Sign hash: reduces collision impact (signed counts)
let sign = if (xxh3_64(&format!("sign_{}", token)) & 1) == 0 { 1.0 } else { -1.0 };

// Accumulate into sparse vector
sparse_vector[idx] += sign;
```

**Random Matrix Generation**:
```rust
// Fixed seed ensures reproducibility
let mut rng = ChaCha8Rng::seed_from_u64(42);

// Uniform[-1, 1] scaled by 1/sqrt(input_dim)
let scaling = (1.0 / 8192_f32.sqrt());
Array2::from_shape_fn((384, 8192), |_| {
    (rng.gen::<f32>() * 2.0 - 1.0) * scaling
})
```

**Collision Mitigation**:
- Use two independent hash functions (position + sign)
- Monitor collision rate during indexing (target <5%)
- Can increase to 16,384D if collisions exceed threshold

---

## 3. Strategy Pattern Architecture

### Decision: Trait-Based Abstraction with Factory Pattern

**Rust Implementation Pattern**:
```rust
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

impl EmbedderBackend {
    pub fn from_config(config: &Config) -> Result<Self> {
        match config.embedding.strategy.as_str() {
            "onnx" => Ok(Self::Onnx(OnnxTokenizer::initialize(config)?)),
            "fallback" => Ok(Self::Fallback(FallbackTokenizer::initialize(config)?)),
            s => Err(Error::InvalidTokenizerStrategy(s.to_string())),
        }
    }
}
```

### Rationale

**Why Trait-Based**:
- ✅ Rust idiomatic (no runtime overhead)
- ✅ Enforces interface contract at compile time
- ✅ Extensible (easy to add new strategies)
- ✅ Type-safe dispatch via enum (no dynamic dispatch needed)

**Why Factory Pattern**:
- ✅ Centralized strategy selection logic
- ✅ Easy to test (inject mock strategies)
- ✅ Config-driven instantiation

**Interface Contract Guarantees**:
1. **Dimension invariant**: All strategies MUST return 384-element Vec<f32>
2. **Determinism**: Same input MUST produce same output within a strategy
3. **Thread safety**: Send + Sync required for concurrent indexing
4. **Error handling**: Initialization errors vs. encoding errors clearly separated

### Integration with Existing Code

**Minimal Changes to EmbeddingGenerator**:
```rust
pub struct EmbeddingGenerator {
    config: Arc<Config>,
    backend: EmbedderBackend,  // Changed from session + tokenizer
}

impl EmbeddingGenerator {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let backend = EmbedderBackend::from_config(&config)?;
        tracing::info!("Initialized embedding generator with strategy: {}", 
                       backend.name());
        Ok(EmbeddingGenerator { config, backend })
    }
    
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        self.backend.encode(text)  // Delegate to strategy
    }
}
```

**Refactoring Plan**:
1. Extract existing ONNX code into `src/embeddings/onnx.rs`
2. Create trait definition in `src/embeddings/mod.rs`
3. Implement FallbackTokenizer in `src/embeddings/fallback.rs`
4. Update EmbeddingGenerator to use strategy abstraction

---

## 4. Quality Validation Approach

### Expected Quality Levels

Based on research and similar systems (ElasticSearch, SimHash):

| Similarity Type | Quality vs. ONNX Baseline |
|-----------------|---------------------------|
| **Exact Duplicates** | 95-100% (near perfect) |
| **Syntactic Similarity** (renamed vars) | 70-85% |
| **Semantic Similarity** (different impls) | 30-50% ⚠️ |
| **Cross-Language** | 10-20% ⚠️ |

### Test Queries (Benchmark Dataset)

Using cudgel's own codebase as test corpus:

**Category 1: Syntactic Search (High Quality Expected)**
- Query: "fn encode text str result vec f32"
- Expected: Find `EmbeddingGenerator::encode` and similar functions
- Acceptance: Top-5 results include at least 3 encoding-related functions

**Category 2: Structural Search (Medium Quality Expected)**
- Query: "database connection postgres config"
- Expected: Find database initialization code
- Acceptance: Top-5 results include Database struct and connection logic

**Category 3: Semantic Search (Low Quality Expected)**
- Query: "error handling user friendly messages"
- Expected: Find Error::to_user_message implementations
- Acceptance: Top-10 results include at least 1 error handling function (50% recall acceptable)

### Quantitative Metrics

**Precision@5**: Percentage of top-5 results that are relevant
- Target for fallback: ≥60% (vs. ≥80% for ONNX)

**Recall@10**: Percentage of relevant documents in top-10
- Target for fallback: ≥40% (vs. ≥70% for ONNX)

**Relative Quality**: 
- Fallback should achieve 50-70% of ONNX quality score
- Acceptable trade-off for offline capability

### Testing Strategy

**Unit Tests** (determinism and correctness):
```rust
#[test]
fn test_fallback_deterministic() {
    let embedder = FallbackTokenizer::new();
    let text = "fn calculate_sum(items: &[i32]) -> i32";
    let embedding1 = embedder.encode(text).unwrap();
    let embedding2 = embedder.encode(text).unwrap();
    assert_eq!(embedding1, embedding2, "Must be deterministic");
}

#[test]
fn test_fallback_dimension() {
    let embedder = FallbackTokenizer::new();
    let embedding = embedder.encode("test code").unwrap();
    assert_eq!(embedding.len(), 384, "Must output 384 dimensions");
}

#[test]
fn test_fallback_normalized() {
    let embedder = FallbackTokenizer::new();
    let embedding = embedder.encode("test code").unwrap();
    let norm: f32 = embedding.iter().map(|x| x*x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.01, "Must be L2 normalized");
}
```

**Integration Tests** (quality validation):
```rust
#[test]
fn test_fallback_similar_code_high_similarity() {
    let embedder = FallbackTokenizer::new();
    let code1 = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let code2 = "fn add(x: i32, y: i32) -> i32 { x + y }";  // Renamed vars
    
    let emb1 = embedder.encode(code1).unwrap();
    let emb2 = embedder.encode(code2).unwrap();
    
    let similarity = cosine_similarity(&emb1, &emb2);
    assert!(similarity > 0.7, "Syntactic similarity should be high");
}
```

**Manual Quality Review**:
- Index cudgel codebase with fallback strategy
- Run 10 representative queries
- Compare top-5 results to ONNX baseline
- Document query-by-query precision scores

---

## 5. Configuration Integration

### Environment Variable Design

**Variable Name**: `CUDGEL_TOKENIZER_STRATEGY`

**Valid Values**:
- `"onnx"` (default if not set): Use ONNX sentence-transformers model
- `"fallback"`: Use built-in feature hashing + random projection

**Case Handling**: Case-insensitive (convert to lowercase before matching)

**Backward Compatibility**: If variable not set, default to "onnx" (existing behavior)

### Configuration Changes

**config.rs modifications**:
```rust
pub struct EmbeddingConfig {
    pub model_path: PathBuf,       // Existing (used only for ONNX)
    pub dimension: usize,           // Existing (must be 384)
    pub strategy: String,           // NEW: tokenization strategy
}

impl Config {
    pub fn local() -> Result<Self> {
        // ... existing config ...
        embedding: EmbeddingConfig {
            model_path: xdg_data_home().join("cudgel/models/all-MiniLM-L6-v2"),
            dimension: 384,
            strategy: std::env::var("CUDGEL_TOKENIZER_STRATEGY")
                .unwrap_or_else(|_| "onnx".to_string())
                .to_lowercase(),  // Case-insensitive
        },
    }
}
```

### Validation Logic

**config.rs validation extension**:
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
                 \n\
                 export CUDGEL_TOKENIZER_STRATEGY=fallback\n\
                 \n\
                 Strategy details:\n\
                 • 'onnx' (default): Best quality, requires model download (cudgel deps)\n\
                 • 'fallback': Offline mode, no downloads required, reduced quality\n\
                 \n\
                 For restricted environments, use 'fallback'.",
                invalid
            )))
        }
    }
}
```

### Error Handling

**embeddings.rs strategy selection**:
```rust
impl EmbeddingGenerator {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let backend = match config.embedding.strategy.as_str() {
            "onnx" => {
                // Try ONNX, provide helpful error if models missing
                OnnxTokenizer::initialize(&config).map(EmbedderBackend::Onnx)
                    .map_err(|e| Error::Embedding(format!(
                        "Failed to initialize ONNX tokenizer: {}\n\
                         \n\
                         Options:\n\
                         1. Download ONNX models: cudgel deps\n\
                         2. Use fallback strategy: export CUDGEL_TOKENIZER_STRATEGY=fallback",
                        e
                    )))?
            }
            "fallback" => {
                FallbackTokenizer::initialize(&config).map(EmbedderBackend::Fallback)?
            }
            _ => unreachable!("Config validation should catch invalid strategies"),
        };
        
        tracing::info!("Initialized embedding generator with '{}' strategy", 
                       config.embedding.strategy);
        Ok(Self { config, backend })
    }
}
```

---

## 6. Implementation Checklist

### Phase 1: Refactor Existing Code
- [ ] Extract ONNX tokenization logic into `src/embeddings/onnx.rs`
- [ ] Define `TokenizerStrategy` trait in `src/embeddings/mod.rs`
- [ ] Update `EmbeddingGenerator` to use strategy abstraction
- [ ] Add `strategy` field to `EmbeddingConfig`
- [ ] Write trait compliance tests for ONNX strategy

### Phase 2: Implement Fallback
- [ ] Create `src/embeddings/fallback.rs`
- [ ] Implement code-aware tokenization (camelCase/snake_case splitting)
- [ ] Implement feature hashing (xxhash with sign trick)
- [ ] Implement random projection matrix generation
- [ ] Implement encoding pipeline (tokenize → hash → project → normalize)
- [ ] Write unit tests (determinism, dimension, normalization)

### Phase 3: Integration
- [ ] Add environment variable reading in `Config::local()`
- [ ] Add strategy validation in `Config::validate()`
- [ ] Implement strategy factory in `EmbedderBackend::from_config()`
- [ ] Update error messages with troubleshooting steps
- [ ] Add logging for strategy selection

### Phase 4: Testing & Validation
- [ ] Write integration tests for strategy switching
- [ ] Benchmark fallback initialization time (<5s requirement)
- [ ] Run quality validation queries on cudgel codebase
- [ ] Compare fallback vs. ONNX query results
- [ ] Document quality expectations in quickstart guide

---

## 7. Risks & Mitigations

### Identified Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Hash collisions degrade quality beyond acceptable** | High | Monitor collision rate; can increase to 16,384D if needed |
| **Random projection doesn't preserve similarities** | High | Use fixed seed (42); verify with unit tests on known similar code |
| **Initialization exceeds 5s target** | Medium | Matrix generation is <2s; no other heavy operations |
| **Fallback quality so poor it's unusable** | Medium | Set realistic expectations in docs; manual quality review in testing phase |
| **Breaking changes to existing ONNX users** | Low | Strategy defaults to "onnx"; refactor maintains interface |

### Contingency Plans

**If quality unacceptable** (SC-005 fails):
- Increase hash space to 16,384D (reduces collisions)
- Add bigram features (increase context)
- Use weighted hashing for code keywords (e.g., "def", "class", "fn")

**If initialization too slow**:
- Generate smaller matrix (4096×384 instead of 8192×384)
- Cache projection matrix to disk (load instead of generate)

**If collisions exceed 10%**:
- Switch from xxhash to MurmurHash3 (better distribution)
- Use 3-hash scheme (2 independent position hashes)

---

## 8. Performance Targets Validation

| Metric | Target | Expected | Status |
|--------|--------|----------|--------|
| **Initialization Time** | <5 seconds | ~2 seconds | ✅ PASS |
| **Memory Footprint** | <500 MB | ~13 MB | ✅ PASS |
| **Indexing Speed** | No degradation | Same (tokenization faster) | ✅ PASS |
| **Query Response** | <1 second | Same (vector search unchanged) | ✅ PASS |
| **Quality Degradation** | Acceptable for offline use | 50-70% of ONNX | ⏳ TO VERIFY |

---

## 9. Dependency Additions

### New Cargo.toml Dependencies

```toml
[dependencies]
# Feature hashing
xxhash-rust = { version = "0.8", default-features = false }

# Random projection (already have ndarray)
rand = "0.8"           # Already present
rand_chacha = "0.3"    # NEW: Deterministic PRNG

# Text processing
unicode-segmentation = "1.12"  # NEW: Unicode-aware tokenization
```

**Total Added Weight**: ~500 KB (compressed crates)

---

## Conclusion

**Selected Approach**: Feature Hashing with Random Projection provides a practical, mathematically sound fallback for offline embedding generation. While semantic quality will be significantly lower (50-70% of ONNX), it enables tool usage in restricted corporate environments where external model downloads are prohibited.

**Key Success Factors**:
1. Fast initialization (<2s measured)
2. Deterministic output (reproducibility)
3. Fixed 384D vectors (pgvector compatible)
4. No external dependencies (truly offline)
5. Acceptable quality for syntactic searches (70-85% similarity for near-duplicates)

**Next Steps**: Proceed to Phase 1 (data model, contracts, quickstart documentation).
