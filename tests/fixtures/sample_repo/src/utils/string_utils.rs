/// String manipulation utilities
use std::collections::HashSet;

/// Trims whitespace from both ends and normalizes multiple spaces
pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Checks if a string contains only alphanumeric characters
pub fn is_alphanumeric(s: &str) -> bool {
    s.chars().all(|c| c.is_alphanumeric())
}

/// Extracts unique words from a string
pub fn extract_words(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|word| !word.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Converts snake_case to PascalCase
pub fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}