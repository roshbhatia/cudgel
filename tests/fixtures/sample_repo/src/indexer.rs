/// Indexing module for demonstration
use std::collections::HashMap;

/// Simple file index entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: usize,
    pub modified: std::time::SystemTime,
}

/// In-memory index for demonstration
pub struct SimpleIndex {
    files: HashMap<String, FileEntry>,
}

impl SimpleIndex {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    
    pub fn add_file(&mut self, path: &str, content: &str) {
        let entry = FileEntry {
            path: path.to_string(),
            size: content.len(),
            modified: std::time::SystemTime::now(),
        };
        
        self.files.insert(path.to_string(), entry);
    }
    
    pub fn get_file(&self, path: &str) -> Option<&FileEntry> {
        self.files.get(path)
    }
    
    pub fn list_files(&self) -> Vec<&FileEntry> {
        self.files.values().collect()
    }
    
    pub fn search(&self, query: &str) -> Vec<&FileEntry> {
        self.files
            .values()
            .filter(|entry| entry.path.contains(query))
            .collect()
    }
}

impl Default for SimpleIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_and_get_file() {
        let mut index = SimpleIndex::new();
        index.add_file("test.txt", "Hello, world!");
        
        let file = index.get_file("test.txt");
        assert!(file.is_some());
        assert_eq!(file.unwrap().size, 13);
    }
    
    #[test]
    fn test_search() {
        let mut index = SimpleIndex::new();
        index.add_file("src/main.rs", "fn main() {}");
        index.add_file("src/lib.rs", "pub fn lib() {}");
        index.add_file("README.md", "# Project");
        
        let results = index.search("src");
        assert_eq!(results.len(), 2);
        
        let results = index.search("main");
        assert_eq!(results.len(), 1);
    }
}