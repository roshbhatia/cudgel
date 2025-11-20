// src/llm/mod.rs
//! LLM client module for generating code summaries.
//!
//! This module provides functionality to generate natural language summaries
//! of code entities using local LLM models via Ollama.

pub mod client;
pub mod prompts;

pub use client::{
    ComponentContext, EntityContext, LlmClient, OllamaClient, RepositoryContext, ServiceHealth,
    SummaryRequest, SummaryResult,
};

use thiserror::Error;

/// Errors that can occur during LLM operations
#[derive(Debug, Error, Clone)]
pub enum LlmError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Generation error: {0}")]
    Generation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl LlmError {
    /// Convert error to user-friendly message with troubleshooting steps
    pub fn to_user_message(&self) -> String {
        match self {
            LlmError::Connection(msg) => {
                format!(
                    "LLM connection error: {}\n\nTroubleshooting:\n- Ensure Ollama is running (ollama serve)\n- Check Ollama is accessible at http://localhost:11434\n- Verify network connectivity",
                    msg
                )
            }
            LlmError::Model(msg) => {
                format!(
                    "LLM model error: {}\n\nTroubleshooting:\n- Install required model: ollama pull llama3.2:3b\n- Check available models: ollama list\n- Ensure sufficient disk space",
                    msg
                )
            }
            LlmError::Generation(msg) => {
                format!(
                    "Summary generation error: {}\n\nTroubleshooting:\n- Check model is loaded correctly\n- Reduce context size if input is too large\n- Retry the operation",
                    msg
                )
            }
            LlmError::Timeout(msg) => {
                format!(
                    "LLM timeout: {}\n\nTroubleshooting:\n- Model may be loading (first run takes longer)\n- Check system resources (CPU/RAM)\n- Increase timeout if processing large inputs",
                    msg
                )
            }
            LlmError::InvalidInput(msg) => {
                format!(
                    "Invalid LLM input: {}\n\nTroubleshooting:\n- Check input format\n- Ensure context is not empty\n- Verify encoding",
                    msg
                )
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, LlmError>;
