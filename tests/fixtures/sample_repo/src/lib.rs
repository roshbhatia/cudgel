pub mod config;
pub mod utils;
pub mod parser;
pub mod indexer;

pub use config::Config;
pub use utils::{normalize_whitespace, is_valid_email};

/// Main library API
pub struct Application {
    config: Config,
}

impl Application {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    pub fn with_default_config() -> Self {
        Self::new(Config::default())
    }
    
    /// Processes user input data
    pub fn process_input(&self, input: &str) -> Result<String, String> {
        let cleaned = normalize_whitespace(input);
        
        if cleaned.is_empty() {
            return Err("Input cannot be empty".to_string());
        }
        
        if cleaned.len() > 1000 {
            return Err("Input too long".to_string());
        }
        
        Ok(format!("Processed: {}", cleaned))
    }
    
    /// Validates user email
    pub fn validate_user_email(&self, email: &str) -> bool {
        is_valid_email(email)
    }
    
    /// Gets the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Creates a new application instance with default settings
pub fn create_app() -> Application {
    Application::with_default_config()
}

/// Creates a new application instance with custom configuration
pub fn create_app_with_config(config: Config) -> Application {
    Application::new(config)
}