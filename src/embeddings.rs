//! Embedding generation for code
//!
//! Uses ONNX Runtime with sentence-transformers/all-MiniLM-L6-v2 model
//! for semantic code embeddings.

use crate::{Config, Error, Result};
use ndarray::Array2;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

/// Embedding generator for semantic code search
///
/// Uses ONNX Runtime with sentence-transformers model for real semantic embeddings.
/// Thread-safe through interior mutability with Mutex.
pub struct EmbeddingGenerator {
    #[allow(dead_code)]
    config: Arc<Config>,
    #[allow(dead_code)]
    dimension: usize,
    session: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    ///
    /// # Arguments
    /// * `config` - Application configuration containing embedding settings
    ///
    /// # Returns
    /// Embedding generator configured with specified dimensions
    ///
    /// # Errors
    /// Returns error if ONNX model or tokenizer cannot be loaded
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let dimension = config.embedding.dimension;
        let model_path = &config.embedding.model_path;

        // Initialize ONNX Runtime environment (only once)
        ort::init()
            .with_name("cudgel")
            .commit()
            .map_err(|e| Error::Embedding(format!("Failed to initialize ONNX runtime: {}", e)))?;

        // Load ONNX model
        let model_file = model_path.join("model.onnx");
        let session = Session::builder()
            .map_err(|e| Error::Embedding(format!("Failed to create session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| Error::Embedding(format!("Failed to set optimization level: {}", e)))?
            .with_intra_threads(4)
            .map_err(|e| Error::Embedding(format!("Failed to set intra threads: {}", e)))?
            .commit_from_file(&model_file)
            .map_err(|e| {
                Error::Embedding(format!(
                    "Failed to load ONNX model from {:?}: {}. \
                     Make sure you've downloaded the model to ./models/all-MiniLM-L6-v2/",
                    model_file, e
                ))
            })?;

        // Load tokenizer
        let tokenizer_file = model_path.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_file).map_err(|e| {
            Error::Embedding(format!(
                "Failed to load tokenizer from {:?}: {}",
                tokenizer_file, e
            ))
        })?;

        Ok(EmbeddingGenerator {
            config,
            dimension,
            session: Mutex::new(session),
            tokenizer,
        })
    }

    /// Encode text into an embedding vector
    ///
    /// Uses ONNX model for semantic embeddings with mean pooling.
    ///
    /// # Arguments
    /// * `text` - Text to encode
    ///
    /// # Returns
    /// Vector of floats (384 dimensions by default)
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize the input text
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| Error::Embedding(format!("Tokenization failed: {}", e)))?;

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        let token_type_ids = encoding.get_type_ids();

        // Convert to i64 for ONNX input
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
        let attention_mask_i64: Vec<i64> =
            attention_mask.iter().map(|&mask| mask as i64).collect();
        let token_type_ids_i64: Vec<i64> =
            token_type_ids.iter().map(|&id| id as i64).collect();

        let seq_len = input_ids.len();

        // Create Value objects from shape and data
        let input_ids_value = Value::from_array(([1, seq_len], input_ids_i64))?;
        let attention_mask_value = Value::from_array(([1, seq_len], attention_mask_i64))?;
        let token_type_ids_value = Value::from_array(([1, seq_len], token_type_ids_i64))?;

        // Run inference (lock the mutex to get mutable access to session)
        let mut session = self
            .session
            .lock()
            .map_err(|e| Error::Embedding(format!("Failed to lock session: {}", e)))?;

        let outputs = session
            .run(ort::inputs![
                "input_ids" => input_ids_value,
                "attention_mask" => attention_mask_value,
                "token_type_ids" => token_type_ids_value,
            ])
            .map_err(|e| Error::Embedding(format!("ONNX inference failed: {}", e)))?;

        // Extract the last_hidden_state output
        let last_hidden_state = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| Error::Embedding(format!("Failed to extract tensor: {}", e)))?;

        // Get shape and data from tensor tuple (shape, data)
        let (shape, data) = last_hidden_state;

        // shape should be [batch_size, seq_len, hidden_dim] = [1, seq_len, 384]
        let shape_dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        if shape_dims.len() != 3 {
            return Err(Error::Embedding(format!(
                "Unexpected output shape: expected 3 dimensions, got {}",
                shape_dims.len()
            )));
        }

        let _batch_size = shape_dims[0];
        let output_seq_len = shape_dims[1];
        let hidden_dim = shape_dims[2];

        // Convert to ndarray for easier manipulation
        let hidden_states = Array2::from_shape_vec(
            (output_seq_len, hidden_dim),
            data.iter().copied().collect(),
        )
        .map_err(|e| Error::Embedding(format!("Failed to reshape tensor: {}", e)))?;

        // Mean pooling: average over the sequence dimension, weighted by attention mask
        let attention_mask_f32: Vec<f32> = attention_mask.iter().map(|&m| m as f32).collect();

        // Compute weighted sum
        let mut pooled = vec![0.0f32; hidden_dim];
        let mut mask_sum = 0.0f32;

        for (i, &mask_val) in attention_mask_f32.iter().enumerate() {
            mask_sum += mask_val;
            for j in 0..hidden_dim {
                pooled[j] += hidden_states[[i, j]] * mask_val;
            }
        }

        // Divide by mask sum to get mean (avoid division by zero)
        let mask_sum = mask_sum.max(1e-9);
        for val in &mut pooled {
            *val /= mask_sum;
        }

        // L2 normalization
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut pooled {
                *val /= norm;
            }
        }

        Ok(pooled)
    }

    /// Encode a code symbol into an embedding
    ///
    /// Combines symbol name, signature, and docstring for richer embeddings.
    ///
    /// # Arguments
    /// * `name` - Symbol name
    /// * `signature` - Optional function/method signature
    /// * `docstring` - Optional documentation string
    ///
    /// # Returns
    /// Vector embedding representing the symbol
    pub fn encode_symbol(
        &self,
        name: &str,
        signature: Option<&str>,
        docstring: Option<&str>,
    ) -> Result<Vec<f32>> {
        let mut parts = vec![name];
        if let Some(sig) = signature {
            parts.push(sig);
        }
        if let Some(doc) = docstring {
            parts.push(doc);
        }

        let text = parts.join(" ");
        self.encode(&text)
    }

    /// Encode a code snippet
    ///
    /// # Arguments
    /// * `code` - Code snippet to encode
    ///
    /// # Returns
    /// Vector embedding of the code
    pub fn encode_code(&self, code: &str) -> Result<Vec<f32>> {
        self.encode(code)
    }

    /// Encode a search query
    ///
    /// # Arguments
    /// * `query` - Natural language search query
    ///
    /// # Returns
    /// Vector embedding of the query
    pub fn encode_query(&self, query: &str) -> Result<Vec<f32>> {
        self.encode(query)
    }
}
