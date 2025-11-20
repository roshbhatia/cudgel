/// Simple parser module for demonstration
pub fn parse_input(input: &str) -> Result<Vec<String>, String> {
    if input.trim().is_empty() {
        return Err("Input cannot be empty".to_string());
    }
    
    Ok(input
        .split_whitespace()
        .map(|s| s.to_string())
        .collect())
}

/// Parse configuration values from key-value pairs
pub fn parse_config_line(line: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("Invalid config line format".to_string());
    }
    
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_input() {
        let result = parse_input("hello world test");
        assert_eq!(result.unwrap(), vec!["hello", "world", "test"]);
    }
    
    #[test]
    fn test_parse_input_empty() {
        let result = parse_input("");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_config_line() {
        let result = parse_config_line("key=value");
        assert_eq!(result.unwrap(), ("key".to_string(), "value".to_string()));
    }
}