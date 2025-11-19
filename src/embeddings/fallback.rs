// src/embeddings/fallback.rs
//
// Fallback tokenization strategy using feature hashing + random projection
//
// This provides offline embedding generation without external model dependencies.
// Algorithm: Code tokenization → xxhash feature hashing → random projection → L2 normalization

use super::TokenizerStrategy;
use crate::{Config, Error, Result};
use ndarray::Array2;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha8Rng,
};

/// Fallback tokenization using feature hashing and random projection
///
/// This strategy provides deterministic embeddings without requiring external models.
/// Trade-off: Lower semantic quality (30-50% degradation) vs. ONNX, but works offline.
///
/// Algorithm:
/// 1. Tokenize code (handle camelCase/snake_case splitting)
/// 2. Hash tokens to sparse 8192D vector (xxhash with signed values)
/// 3. Random projection to 384D (fixed seed for reproducibility)
/// 4. L2 normalization
///
/// Performance characteristics:
/// - Initialization: <2 seconds
/// - Encoding: <5ms per text
/// - Memory: ~12.5 MB (projection matrix)
pub struct FallbackTokenizer {
    /// Random projection matrix (384 × 8192)
    projection_matrix: Array2<f32>,
    /// Hash space dimension
    hash_dimension: usize,
}

impl TokenizerStrategy for FallbackTokenizer {
    fn initialize(_config: &Config) -> Result<Self> {
        const HASH_DIM: usize = 8192;
        const EMBED_DIM: usize = 384;
        const SEED: u64 = 42;

        tracing::debug!(
            "Initializing fallback tokenizer with {}x{} projection matrix",
            EMBED_DIM,
            HASH_DIM
        );

        // Generate deterministic random projection matrix
        let mut rng = ChaCha8Rng::seed_from_u64(SEED);
        let scaling = 1.0 / (HASH_DIM as f32).sqrt(); // Johnson-Lindenstrauss lemma

        let projection_matrix = Array2::from_shape_fn((EMBED_DIM, HASH_DIM), |_| {
            // Generate random f32 in [0, 1), then map to [-1, 1)
            let random_f32 = (rng.next_u32() as f32) / (u32::MAX as f32);
            (random_f32 * 2.0 - 1.0) * scaling
        });

        tracing::debug!("Fallback tokenizer initialized successfully");

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
                "Invalid projection matrix shape: expected (384, {}), got ({}, {})",
                self.hash_dimension, shape[0], shape[1]
            )));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "fallback"
    }
}

// Implementation details
impl FallbackTokenizer {
    /// Tokenize code text into individual tokens
    ///
    /// Handles:
    /// - Whitespace splitting
    /// - camelCase splitting (getUserName → [get, User, Name])
    /// - snake_case splitting (get_user_name → [get, user, name])
    /// - Lowercase normalization
    fn tokenize_code(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .flat_map(|word| self.split_identifier(word))
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Split identifiers by camelCase and snake_case conventions
    fn split_identifier(&self, s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut prev_lower = false;

        for ch in s.chars() {
            if ch == '_' {
                // snake_case separator
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                prev_lower = false;
            } else if ch.is_alphanumeric() {
                if ch.is_uppercase() {
                    // camelCase transition
                    if !current.is_empty() && prev_lower {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    current.push(ch);
                    prev_lower = false;
                } else {
                    current.push(ch);
                    prev_lower = ch.is_lowercase();
                }
            } else {
                // Non-alphanumeric, non-underscore character - treat as delimiter
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                prev_lower = false;
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        // If no splitting occurred, return original
        if tokens.is_empty() {
            vec![s.to_string()]
        } else {
            tokens
        }
    }

    /// Create sparse feature vector using signed hash trick
    fn create_feature_vector(&self, tokens: &[String]) -> Vec<f32> {
        use xxhash_rust::xxh3::xxh3_64;

        let mut sparse = vec![0.0; self.hash_dimension];

        for token in tokens {
            // Primary hash for index
            let idx = (xxh3_64(token.as_bytes()) % self.hash_dimension as u64) as usize;

            // Sign hash for ±1
            let sign_bytes = format!("sign_{}", token);
            let sign = if (xxh3_64(sign_bytes.as_bytes()) & 1) == 0 {
                1.0
            } else {
                -1.0
            };

            sparse[idx] += sign;
        }

        sparse
    }

    /// Project sparse vector to 384D using random projection matrix
    fn project_to_embedding(&self, sparse: &[f32]) -> Vec<f32> {
        use ndarray::Array1;

        let input = Array1::from(sparse.to_vec());
        let output = self.projection_matrix.dot(&input);
        output.to_vec()
    }

    /// L2 normalize vector to unit length
    fn normalize(&self, vec: &mut [f32]) {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        } else {
            // Handle zero vector (empty input) - create uniform distribution
            let uniform_value = 1.0 / (vec.len() as f32).sqrt();
            vec.iter_mut().for_each(|x| *x = uniform_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_identifier_camelcase() {
        let tokenizer = FallbackTokenizer {
            projection_matrix: Array2::zeros((384, 8192)),
            hash_dimension: 8192,
        };

        let result = tokenizer.split_identifier("getUserName");
        assert_eq!(result, vec!["get", "User", "Name"]);
    }

    #[test]
    fn test_split_identifier_snakecase() {
        let tokenizer = FallbackTokenizer {
            projection_matrix: Array2::zeros((384, 8192)),
            hash_dimension: 8192,
        };

        let result = tokenizer.split_identifier("get_user_name");
        assert_eq!(result, vec!["get", "user", "name"]);
    }

    #[test]
    fn test_tokenize_code() {
        let tokenizer = FallbackTokenizer {
            projection_matrix: Array2::zeros((384, 8192)),
            hash_dimension: 8192,
        };

        let result = tokenizer.tokenize_code("fn getUserName() -> String");
        eprintln!("Tokenized result: {:?}", result);
        assert!(result.contains(&"fn".to_string()));
        assert!(result.contains(&"get".to_string()));
        assert!(result.contains(&"user".to_string()));
        assert!(result.contains(&"name".to_string()));
    }
}
