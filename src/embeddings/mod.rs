// src/embeddings/mod.rs
//
// Tokenization strategy abstraction for pluggable embedding generation

use crate::{Config, Result};

/// Strategy for tokenizing text and generating embeddings
///
/// All implementations must satisfy these invariants:
/// 1. **Fixed Dimension**: encode() always returns Vec<f32> with exactly 384 elements
/// 2. **Unit Vectors**: Output vectors are L2 normalized (norm = 1.0 ± 0.01)
/// 3. **Determinism**: Same text input produces identical output within a strategy
/// 4. **Thread Safety**: All methods safe to call from multiple threads (Send + Sync)
/// 5. **Error Transparency**: Errors include actionable troubleshooting information
pub trait TokenizerStrategy: Send + Sync {
    /// Initialize the strategy with given configuration
    ///
    /// # Errors
    /// Returns error if initialization fails (missing models, invalid config, etc.)
    fn initialize(config: &Config) -> Result<Self>
    where
        Self: Sized;

    /// Encode text into a 384-dimensional embedding vector
    ///
    /// # Invariants
    /// - Output length is always 384
    /// - Output vector is L2 normalized
    /// - Same input always produces identical output
    ///
    /// # Errors
    /// Returns error if encoding fails
    fn encode(&self, text: &str) -> Result<Vec<f32>>;

    /// Validate that strategy is ready to encode
    ///
    /// # Errors
    /// Returns error if strategy cannot encode (corrupted state, missing resources)
    fn validate(&self) -> Result<()>;

    /// Return human-readable strategy name for logging
    fn name(&self) -> &'static str;
}

/// Type-safe container for strategy instances
///
/// Uses enum dispatch for zero-cost abstraction (compile-time polymorphism)
pub enum EmbedderBackend {
    Onnx(OnnxTokenizer),
    Fallback(FallbackTokenizer),
}

impl EmbedderBackend {
    /// Factory method to create backend from configuration
    ///
    /// # Errors
    /// - InvalidTokenizerStrategy if strategy name not recognized
    /// - Strategy initialization errors (missing models, etc.)
    pub fn from_config(config: &Config) -> Result<Self> {
        match config.embedding.strategy.as_str() {
            "onnx" => Ok(Self::Onnx(OnnxTokenizer::initialize(config)?)),
            "fallback" => Ok(Self::Fallback(FallbackTokenizer::initialize(config)?)),
            s => Err(crate::Error::InvalidTokenizerStrategy(s.to_string())),
        }
    }

    /// Encode text using active strategy
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Onnx(t) => t.encode(text),
            Self::Fallback(t) => t.encode(text),
        }
    }

    /// Get name of active strategy
    pub fn name(&self) -> &'static str {
        match self {
            Self::Onnx(t) => t.name(),
            Self::Fallback(t) => t.name(),
        }
    }

    /// Validate active strategy
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Onnx(t) => t.validate(),
            Self::Fallback(t) => t.validate(),
        }
    }
}

// Module declarations (implementations in separate files)
pub mod fallback;
pub mod onnx;

// Re-export strategy implementations for testing
pub use self::fallback::FallbackTokenizer;
pub use self::onnx::OnnxTokenizer;

// Temporary backward compatibility alias (Phase 4 will replace usages)
// This allows existing code in indexer.rs and query.rs to compile
pub use self::onnx::OnnxTokenizer as EmbeddingGenerator;
