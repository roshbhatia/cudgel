// src/llm/prompts.rs
//! Prompt templates for LLM-based code summarization.
//!
//! These templates are used to generate contextually appropriate summaries
//! for repositories, components, and code entities.

/// Prompt template for repository-level architecture summaries
///
/// Variables: name, languages, modules, file_count, entity_count, patterns
pub const REPOSITORY_PROMPT: &str = r#"You are a technical documentation expert analyzing a code repository.

Repository Information:
- Name: {name}
- Languages: {languages}
- Top Modules: {modules}
- Files: {file_count}
- Entities: {entity_count}
- Key Patterns: {patterns}

Task: Generate a concise, technical summary (3-5 sentences) of this repository's architecture.

Focus on:
1. Primary purpose and domain
2. Main architectural patterns (e.g., MVC, layered, microservices)
3. Key technologies and frameworks
4. Overall organization strategy

Output only the summary text, no additional formatting or metadata."#;

/// Prompt template for component/module-level summaries
///
/// Variables: name, type, file_count, dependencies, exported_entities, patterns
pub const COMPONENT_PROMPT: &str = r#"You are a technical documentation expert analyzing a code component.

Component Information:
- Name: {name}
- Type: {type}
- Files: {file_count}
- Dependencies: {dependencies}
- Public API: {exported_entities}
- Patterns: {patterns}

Task: Generate a concise summary (2-3 sentences) of this component's purpose and role.

Focus on:
1. Primary responsibility
2. How it fits into the larger system
3. Key APIs or interfaces it exposes

Output only the summary text, no additional formatting or metadata."#;

/// Prompt template for entity-level (class, function) summaries
///
/// Variables: name, type, file_path, signature, dependencies, visibility, code_snippet
pub const ENTITY_PROMPT: &str = r#"You are a technical documentation expert analyzing a code entity.

Entity Information:
- Name: {name}
- Type: {type}
- Location: {file_path}
- Signature: {signature}
- Dependencies: {dependencies}
- Visibility: {visibility}

Code:
```
{code_snippet}
```

Task: Generate a single-sentence summary of what this {type} does.

Focus on:
1. Primary purpose or responsibility
2. Key inputs and outputs (if applicable)
3. Important side effects or state changes

Output only the summary text, no additional formatting or metadata."#;

/// Prompt template for pattern analysis (cross-cutting concerns)
///
/// Variables: pattern, entities
pub const PATTERN_ANALYSIS_PROMPT: &str = r#"You are a technical documentation expert analyzing code patterns.

Pattern: {pattern}
Entities using this pattern: {entities}

Task: Generate a 2-3 sentence explanation of how this pattern is used across these entities.

Focus on:
1. Common usage patterns or idioms
2. Consistency or variations in implementation
3. Architectural implications

Output only the analysis text, no additional formatting or metadata."#;

/// Fill a prompt template with provided values
///
/// # Arguments
/// * `template` - The prompt template (one of the constants above)
/// * `values` - Key-value pairs to substitute in the template
///
/// # Examples
/// ```
/// use cudgel::llm::prompts::{REPOSITORY_PROMPT, fill_template};
/// use std::collections::HashMap;
///
/// let mut values = HashMap::new();
/// values.insert("name".to_string(), "MyProject".to_string());
/// values.insert("languages".to_string(), "Rust, Python".to_string());
/// // ... other values
///
/// let prompt = fill_template(REPOSITORY_PROMPT, &values);
/// ```
pub fn fill_template(template: &str, values: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    
    for (key, value) in values {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_fill_template() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "TestProject".to_string());
        values.insert("languages".to_string(), "Rust".to_string());
        values.insert("modules".to_string(), "core, api, cli".to_string());
        values.insert("file_count".to_string(), "42".to_string());
        values.insert("entity_count".to_string(), "150".to_string());
        values.insert("patterns".to_string(), "async, database".to_string());

        let result = fill_template(REPOSITORY_PROMPT, &values);

        assert!(result.contains("TestProject"));
        assert!(result.contains("Rust"));
        assert!(result.contains("42"));
        assert!(!result.contains("{name}"));
        assert!(!result.contains("{languages}"));
    }

    #[test]
    fn test_component_prompt_has_required_placeholders() {
        assert!(COMPONENT_PROMPT.contains("{name}"));
        assert!(COMPONENT_PROMPT.contains("{type}"));
        assert!(COMPONENT_PROMPT.contains("{dependencies}"));
    }

    #[test]
    fn test_entity_prompt_has_required_placeholders() {
        assert!(ENTITY_PROMPT.contains("{name}"));
        assert!(ENTITY_PROMPT.contains("{type}"));
        assert!(ENTITY_PROMPT.contains("{code_snippet}"));
    }

    #[test]
    fn test_pattern_analysis_prompt_has_required_placeholders() {
        assert!(PATTERN_ANALYSIS_PROMPT.contains("{pattern}"));
        assert!(PATTERN_ANALYSIS_PROMPT.contains("{entities}"));
    }
}
