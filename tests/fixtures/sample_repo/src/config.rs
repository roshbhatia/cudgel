use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration structure for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub server_port: u16,
    pub log_level: String,
    pub features: HashMap<String, bool>,
}

impl Default for Config {
    fn default() -> Self {
        let mut features = HashMap::new();
        features.insert("auth".to_string(), true);
        features.insert("cache".to_string(), false);
        
        Self {
            database_url: "sqlite://app.db".to_string(),
            server_port: 8080,
            log_level: "info".to_string(),
            features,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_database(mut self, url: impl Into<String>) -> Self {
        self.database_url = url.into();
        self
    }
    
    pub fn with_port(mut self, port: u16) -> Self {
        self.server_port = port;
        self
    }
    
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.features.get(feature).copied().unwrap_or(false)
    }
}