// src/embeddings/onnx.rs
//
// ONNX tokenization strategy (placeholder - will be implemented in Phase 4)

use super::TokenizerStrategy;
use crate::{Config, Error, Result};
use std::sync::Arc;

/// ONNX-based tokenization using sentence-transformers model
///
/// This is a placeholder. Full implementation in Phase 4 (User Story 2).
pub struct OnnxTokenizer;

impl OnnxTokenizer {
    /// Backward compatibility constructor for old code
    ///
    /// This allows existing code in indexer.rs and query.rs to compile
    /// until Phase 4 refactoring is complete.
    pub fn new(_config: Arc<Config>) -> Result<Self> {
        Self::initialize(&_config)
    }

    /// Backward compatibility method for indexer.rs
    pub fn encode_symbol(
        &self,
        _name: &str,
        _signature: Option<&str>,
        _docstring: Option<&str>,
    ) -> Result<Vec<f32>> {
        Err(Error::Embedding(
            "ONNX tokenizer not yet refactored. This will be implemented in Phase 4.".to_string(),
        ))
    }

    /// Backward compatibility method for query.rs
    pub fn encode_query(&self, _query: &str) -> Result<Vec<f32>> {
        Err(Error::Embedding(
            "ONNX tokenizer not yet refactored. This will be implemented in Phase 4.".to_string(),
        ))
    }
}

impl TokenizerStrategy for OnnxTokenizer {
    fn initialize(_config: &Config) -> Result<Self> {
        // Placeholder: Will extract existing ONNX code in Phase 4
        Err(Error::Embedding(
            "ONNX tokenizer not yet refactored. This will be implemented in Phase 4.".to_string(),
        ))
    }

    fn encode(&self, _text: &str) -> Result<Vec<f32>> {
        Err(Error::Embedding("Not implemented".to_string()))
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "onnx"
    }
}
