# Data Model: Fallback Tokenization Strategy

**Feature**: 001-fallback-tokenization  
**Date**: 2025-11-19  
**Purpose**: Define entities and their relationships for pluggable tokenization strategies

---

## Overview

This feature introduces a strategy pattern for tokenization, allowing the embedding generator to switch between ONNX (pre-trained models) and fallback (deterministic hashing) approaches based on environment configuration.

---

## Entity: TokenizerStrategy (Trait)

### Purpose
Defines the interface for all tokenization strategy implementations to ensure consistent embedding generation behavior.

### Type
Rust trait (compile-time polymorphism)

### Attributes (Trait Methods)

| Method | Signature | Purpose | Constraints |
|--------|-----------|---------|-------------|
| `initialize` | `fn initialize(config: &Config) -> Result<Self>` | Create and initialize strategy | Must validate required resources exist |
| `encode` | `fn encode(&self, text: &str) -> Result<Vec<f32>>` | Generate embedding from text | Must return exactly 384 elements |
| `validate` | `fn validate(&self) -> Result<()>` | Verify strategy is ready | Called after initialization |
| `name` | `fn name(&self) -> &'static str` | Return strategy identifier | Used for logging and diagnostics |

### Trait Constraints
- `Send + Sync`: Must be thread-safe for concurrent indexing operations
- `'static` lifetime for `name()` return value (no dynamic strings)

### Implementations

#### 1. OnnxTokenizer (Refactored from existing code)

**Purpose**: High-quality semantic embeddings using sentence-transformers ONNX model.

**Attributes**:
- `session: Mutex<Session>` - ONNX Runtime session (mutable for inference)
- `tokenizer: Tokenizer` - HuggingFace tokenizers::Tokenizer
- `config: Arc<Config>` - Reference to application config

**Initialization**:
- Load ONNX model from `config.embedding.model_path/model.onnx`
- Load tokenizer from `config.embedding.model_path/tokenizer.json`
- Validate both files exist before loading
- Initialize ONNX Runtime environment

**Encoding Pipeline**:
1. Tokenize text using HuggingFace tokenizer
2. Generate input_ids, attention_mask, token_type_ids
3. Run ONNX inference to get last_hidden_state
4. Apply mean pooling weighted by attention_mask
5. L2 normalize to unit vector

**Validation**:
- Check ONNX session is initialized
- Check tokenizer is loaded
- Verify model files still accessible

**Name**: `"onnx"`

---

#### 2. FallbackTokenizer (New implementation)

**Purpose**: Offline embedding generation using feature hashing and random projection.

**Attributes**:
- `projection_matrix: Array2<f32>` - Fixed 8192×384 random projection matrix
- `hash_dimension: usize` - Intermediate hash space size (8192)
- `seed: u64` - Fixed random seed for reproducibility (42)

**Initialization**:
- Generate 8192×384 random projection matrix using ChaCha8Rng with seed 42
- Scale matrix values by `1.0 / sqrt(8192)` (Johnson-Lindenstrauss)
- Allocate matrix memory (~12.5 MB)
- Expected time: <2 seconds

**Encoding Pipeline**:
1. **Tokenize**: Split text into code tokens (handle camelCase, snake_case, operators)
2. **Hash**: Map each token to (index, sign) using xxhash
   - Primary hash: `xxh3_64(token) % 8192` → index
   - Sign hash: `xxh3_64("sign_" + token) & 1` → ±1.0
3. **Accumulate**: Build sparse 8192D vector with signed counts
4. **Project**: Multiply by projection_matrix (8192×384)
5. **Normalize**: L2 normalize to unit vector

**Validation**:
- Check projection_matrix is initialized (non-null)
- Verify dimensions (8192×384)
- Always returns Ok (no external dependencies)

**Name**: `"fallback"`

---

### Invariants (All Implementations Must Guarantee)

1. **Fixed Dimension**: `encode()` always returns `Vec<f32>` with exactly 384 elements
2. **Unit Vectors**: Output vectors are L2 normalized (norm = 1.0 ± 0.01)
3. **Determinism**: Same text input produces identical output within a strategy
4. **Thread Safety**: All methods safe to call from multiple threads concurrently
5. **Error Transparency**: Errors include actionable troubleshooting information

---

## Entity: EmbedderBackend (Enum)

### Purpose
Type-safe container for strategy instances, enabling compile-time dispatch.

### Variants

```rust
pub enum EmbedderBackend {
    Onnx(OnnxTokenizer),
    Fallback(FallbackTokenizer),
}
```

### Factory Method

```rust
impl EmbedderBackend {
    pub fn from_config(config: &Config) -> Result<Self> {
        match config.embedding.strategy.as_str() {
            "onnx" => Ok(Self::Onnx(OnnxTokenizer::initialize(config)?)),
            "fallback" => Ok(Self::Fallback(FallbackTokenizer::initialize(config)?)),
            s => Err(Error::InvalidTokenizerStrategy(s.to_string())),
        }
    }
    
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Onnx(t) => t.encode(text),
            Self::Fallback(t) => t.encode(text),
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Onnx(t) => t.name(),
            Self::Fallback(t) => t.name(),
        }
    }
}
```

### Relationships
- Contains exactly one TokenizerStrategy implementation
- Created by `EmbeddingGenerator::new()`
- Lifetime tied to EmbeddingGenerator instance

---

## Entity: EmbeddingConfig (Extended)

### Purpose
Configuration for embedding generation, including strategy selection.

### Attributes

| Field | Type | Purpose | Source |
|-------|------|---------|--------|
| `model_path` | `PathBuf` | Path to ONNX model files | XDG data home + "cudgel/models/all-MiniLM-L6-v2" |
| `dimension` | `usize` | Embedding vector size | Hardcoded 384 |
| `strategy` | `String` | Active tokenization strategy | **NEW**: `CUDGEL_TOKENIZER_STRATEGY` env var (default "onnx") |

### Validation Rules

| Rule | Validation Logic | Error Message |
|------|------------------|---------------|
| Strategy validity | `strategy in ["onnx", "fallback"]` | "Invalid tokenization strategy '{strategy}'. Valid: 'onnx', 'fallback'" |
| Dimension | `dimension == 384` | "Embedding dimension must be 384" |
| ONNX model path | If strategy=="onnx", model_path must contain model.onnx | "ONNX model not found at {model_path}" |

### State Transitions

```
Configuration Loaded (from env vars)
    ↓
Strategy Validation (Config::validate)
    ↓
Strategy Initialization (EmbedderBackend::from_config)
    ↓
Ready for Encoding (EmbeddingGenerator::encode)
```

### Backward Compatibility

**Pre-Feature Behavior**:
- No `strategy` field
- Always uses ONNX

**Post-Feature Behavior**:
- `strategy` defaults to "onnx" if env var not set
- Existing users experience no change (ONNX remains default)
- New field added with backward-compatible default

---

## Entity: EmbeddingGenerator (Modified)

### Purpose
Primary interface for generating code embeddings, now strategy-aware.

### Attributes

| Field | Type | Purpose | Change |
|-------|------|---------|--------|
| `config` | `Arc<Config>` | Application configuration | Unchanged |
| `backend` | `EmbedderBackend` | Active tokenization strategy | **NEW**: Replaces `session` + `tokenizer` |
| ~~`session`~~ | ~~`Mutex<Session>`~~ | ONNX session | **REMOVED**: Moved to OnnxTokenizer |
| ~~`tokenizer`~~ | ~~`Tokenizer`~~ | HuggingFace tokenizer | **REMOVED**: Moved to OnnxTokenizer |

### Public Methods

| Method | Signature | Behavior | Change |
|--------|-----------|----------|--------|
| `new` | `fn new(config: Arc<Config>) -> Result<Self>` | Initialize strategy from config | **MODIFIED**: Uses EmbedderBackend factory |
| `encode` | `fn encode(&self, text: &str) -> Result<Vec<f32>>` | Generate embedding | **MODIFIED**: Delegates to backend |
| `encode_symbol` | `fn encode_symbol(&self, name: &str, sig: Option<&str>, doc: Option<&str>) -> Result<Vec<f32>>` | Encode code symbol | Unchanged (uses encode internally) |
| `encode_code` | `fn encode_code(&self, code: &str) -> Result<Vec<f32>>` | Encode code snippet | Unchanged |
| `encode_query` | `fn encode_query(&self, query: &str) -> Result<Vec<f32>>` | Encode search query | Unchanged |

### Initialization Flow

```
EmbeddingGenerator::new(config)
    ↓
Read config.embedding.strategy
    ↓
EmbedderBackend::from_config(config)
    ↓ [match strategy]
    ├─ "onnx" → OnnxTokenizer::initialize()
    │              ↓
    │          Load ONNX model + tokenizer
    │              ↓
    │          Return EmbedderBackend::Onnx
    │
    └─ "fallback" → FallbackTokenizer::initialize()
                       ↓
                   Generate projection matrix
                       ↓
                   Return EmbedderBackend::Fallback
    ↓
Log active strategy name
    ↓
Return EmbeddingGenerator with backend
```

---

## Relationships Between Entities

```
Config
  ↓ contains
EmbeddingConfig
  ↓ configures
EmbeddingGenerator
  ↓ owns
EmbedderBackend (enum)
  ↓ contains one of
  ├─ OnnxTokenizer (implements TokenizerStrategy)
  └─ FallbackTokenizer (implements TokenizerStrategy)
```

### Cardinality
- 1 Config : 1 EmbeddingConfig (embedded)
- 1 EmbeddingGenerator : 1 EmbedderBackend (owned)
- 1 EmbedderBackend : 1 TokenizerStrategy implementation (owned)

### Lifecycle
- Config: Application lifetime (Arc-shared)
- EmbeddingGenerator: Created once per application run
- EmbedderBackend: Lifetime tied to EmbeddingGenerator
- Strategy instance: Lifetime tied to EmbedderBackend

---

## Database Schema Changes

**No database changes required.** Embedding vectors remain:
- Type: `vector(384)` (pgvector)
- Storage: `embeddings` column in `symbols` table

### Metadata Storage (Optional Future Enhancement)

If tracking strategy used for embeddings:

```sql
-- Optional: Add metadata column to symbols table
ALTER TABLE symbols ADD COLUMN embedding_metadata JSONB;

-- Example metadata
{
  "strategy": "fallback",
  "version": "1.0",
  "timestamp": 1700000000
}
```

**Not implemented in this feature** (out of scope), but schema is forward-compatible.

---

## Code Structure Changes

### File Organization

```
src/
├── embeddings.rs              → Modified to use backend abstraction
├── embeddings/                → NEW module
│   ├── mod.rs                → Public interface, trait, backend enum, factory
│   ├── onnx.rs               → Extracted ONNX implementation
│   └── fallback.rs           → NEW fallback implementation
├── config.rs                 → Modified to add strategy field
└── error.rs                  → Modified to add InvalidTokenizerStrategy variant
```

### Module Visibility

```rust
// src/embeddings/mod.rs
pub trait TokenizerStrategy: Send + Sync { /* ... */ }
pub enum EmbedderBackend { /* ... */ }

// src/embeddings/onnx.rs
pub(crate) struct OnnxTokenizer { /* ... */ }  // Not public, used via trait

// src/embeddings/fallback.rs
pub(crate) struct FallbackTokenizer { /* ... */ }  // Not public, used via trait
```

**Design Principle**: Only `EmbedderBackend` is public. Concrete strategy types are internal implementation details.

---

## Error Handling

### New Error Variants

```rust
pub enum Error {
    // ... existing variants ...
    
    /// Invalid tokenization strategy specified
    InvalidTokenizerStrategy(String),
    
    /// Strategy initialization failed
    StrategyInitialization { strategy: String, reason: String },
}
```

### Error Messages

**Invalid Strategy**:
```
Invalid tokenization strategy 'mlflow'. Valid options: 'onnx', 'fallback'.

Set via environment variable:
  export CUDGEL_TOKENIZER_STRATEGY=fallback

Strategy details:
  • 'onnx' (default): Best quality, requires model download (cudgel deps)
  • 'fallback': Offline mode, no downloads required, reduced quality
```

**ONNX Initialization Failed**:
```
Failed to initialize ONNX tokenizer: ONNX model not found at /path/to/models/

Options:
  1. Download ONNX models: cudgel deps
  2. Use fallback strategy: export CUDGEL_TOKENIZER_STRATEGY=fallback
```

---

## Testing Considerations

### Unit Test Coverage

**Per Strategy**:
- Dimension correctness (always 384)
- Determinism (same input → same output)
- Normalization (L2 norm = 1.0)
- Thread safety (concurrent encoding)

**Integration**:
- Strategy switching via environment variable
- Backward compatibility (no env var → ONNX default)
- Error handling (invalid strategy, missing models)

### Test Data

**Sample Code Snippets** (from cudgel codebase):
```rust
"fn encode(&self, text: &str) -> Result<Vec<f32>>"
"pub struct EmbeddingGenerator { config: Arc<Config> }"
"let embedding = self.encode(query)?;"
```

**Expected Behaviors**:
- Syntactically similar code → cosine similarity >0.7
- Unrelated code → cosine similarity <0.3
- Exact duplicates → cosine similarity ~1.0

---

## Summary

This data model introduces a **strategy pattern** for tokenization, enabling:
1. **Pluggability**: Easy to add new strategies (e.g., "hybrid" in future)
2. **Type Safety**: Enum dispatch eliminates runtime polymorphism overhead
3. **Backward Compatibility**: Existing ONNX behavior preserved as default
4. **Testability**: Clear interfaces enable comprehensive unit testing

Key design decisions:
- Trait-based abstraction (Rust idiomatic)
- Enum dispatch (compile-time polymorphism)
- Factory pattern (centralized instantiation)
- Fixed 384D constraint (enforced by trait invariant)
