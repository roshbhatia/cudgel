// src/kg/query.rs
//! Natural language query interface for knowledge graph.
//!
//! Provides fuzzy entity matching and relationship query parsing to enable
//! queries like "what does Parser interact with?" or "show dependencies of Config".

use crate::kg::{
    client::KgClient, CodeEntity, EntityMatch, EntityRelationships, KgError, Result,
};
use regex::Regex;

/// Fuzzy matcher for finding entities by partial/ambiguous names
pub struct EntityMatcher {
    threshold: f64, // Minimum similarity score (0.0 to 1.0)
}

impl EntityMatcher {
    /// Create a new entity matcher with default threshold (0.85)
    pub fn new() -> Self {
        Self {
            threshold: 0.85,
        }
    }

    /// Create a new entity matcher with custom threshold
    pub fn with_threshold(threshold: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(KgError::InvalidInput(
                "Threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(Self { threshold })
    }

    /// Find entities by name using fuzzy matching
    ///
    /// Returns matches sorted by confidence score (highest first).
    /// Uses Jaro-Winkler distance for fuzzy string matching.
    pub async fn find_entities_by_name<T: KgClient + ?Sized>(
        &self,
        client: &T,
        repository_id: i32,
        query_name: &str,
    ) -> Result<Vec<EntityMatch>> {
        if query_name.is_empty() {
            return Err(KgError::InvalidInput(
                "Query name cannot be empty".to_string(),
            ));
        }

        // Get all entities in the repository
        let components = client.get_components(&repository_id).await?;
        let mut all_entities = Vec::new();

        if let Some(_component) = components.into_iter().next() {
            // Get entities by searching for all names (we'll get them through the repository)
            // This is a workaround since we don't have a direct list_entities method
            let entities = client
                .find_entities_by_name(&repository_id, "")
                .await
                .unwrap_or_default();
            all_entities.extend(entities);
             // We only need to do this once to get all entities
        }

        // If no components, try to get all entity names and then fetch them
        if all_entities.is_empty() {
            let names = client.get_all_entity_names(&repository_id).await?;
            for name in names {
                let entities = client.find_entities_by_name(&repository_id, &name).await?;
                all_entities.extend(entities);
            }
            // Deduplicate by ID
            all_entities.sort_by_key(|e| e.id);
            all_entities.dedup_by_key(|e| e.id);
        }

        // Compute similarity scores
        let mut matches: Vec<EntityMatch> = all_entities
            .into_iter()
            .filter_map(|entity| {
                let confidence = self.compute_similarity(&entity.name, query_name);
                if confidence >= self.threshold {
                    Some(EntityMatch { entity, confidence })
                } else {
                    None
                }
            })
            .collect();

        // Sort by confidence (highest first)
        matches.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// Compute similarity between two strings using Jaro-Winkler distance
    fn compute_similarity(&self, s1: &str, s2: &str) -> f64 {
        // Case-insensitive comparison
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();

        // Exact match
        if s1_lower == s2_lower {
            return 1.0;
        }

        // Jaro-Winkler similarity
        strsim::jaro_winkler(&s1_lower, &s2_lower)
    }
}

impl Default for EntityMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Intent extracted from a natural language query
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// Find what an entity depends on
    Dependencies { entity_name: String },
    /// Find what depends on an entity
    Dependents { entity_name: String },
    /// Find all relationships for an entity
    AllRelationships { entity_name: String },
    /// Find what an entity uses
    Uses { entity_name: String },
    /// Find what uses an entity
    UsedBy { entity_name: String },
    /// Find what functions an entity calls
    Calls { entity_name: String },
    /// Find what functions call an entity
    CalledBy { entity_name: String },
    /// Find what an entity implements
    Implements { entity_name: String },
    /// Find what entities implement this
    ImplementedBy { entity_name: String },
}

/// Type alias for pattern matching function
type PatternFn = Box<dyn Fn(&str) -> QueryIntent + Send + Sync>;

/// Type alias for pattern list
type PatternList = Vec<(Regex, PatternFn)>;

/// Parser for natural language relationship queries
pub struct QueryParser {
    patterns: PatternList,
}

impl QueryParser {
    /// Create a new query parser with default patterns
    pub fn new() -> Self {
        let patterns: PatternList = vec![
            // Most specific patterns first to avoid false matches
            
            // Dependencies - "what does X depend on"
            (
                Regex::new(r"(?i)what\s+(?:does|do)\s+(\w+)\s+depend").unwrap(),
                Box::new(|entity: &str| QueryIntent::Dependencies {
                    entity_name: entity.to_string(),
                }),
            ),
            // Dependencies - "dependencies of X"
            (
                Regex::new(r"(?i)(?:show\s+)?dependencies\s+(?:of|for)\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::Dependencies {
                    entity_name: entity.to_string(),
                }),
            ),
            
            // Dependents - "what depends on X"
            (
                Regex::new(r"(?i)what\s+depends\s+on\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::Dependents {
                    entity_name: entity.to_string(),
                }),
            ),
            // Dependents - "dependents of X"
            (
                Regex::new(r"(?i)(?:show\s+)?dependents\s+(?:of|for)\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::Dependents {
                    entity_name: entity.to_string(),
                }),
            ),
            // Dependencies - "X depends on" (less specific, after "what depends on X")
            (
                Regex::new(r"(?i)(\w+)\s+depend(?:s|encies)?\s+on").unwrap(),
                Box::new(|entity: &str| QueryIntent::Dependencies {
                    entity_name: entity.to_string(),
                }),
            ),
            
            // Uses - "what does X use"
            (
                Regex::new(r"(?i)what\s+(?:does|do)\s+(\w+)\s+use").unwrap(),
                Box::new(|entity: &str| QueryIntent::Uses {
                    entity_name: entity.to_string(),
                }),
            ),
            // Uses - "X uses what"
            (
                Regex::new(r"(?i)(\w+)\s+uses?\s+what").unwrap(),
                Box::new(|entity: &str| QueryIntent::Uses {
                    entity_name: entity.to_string(),
                }),
            ),
            // Used by - "what uses X"
            (
                Regex::new(r"(?i)what\s+uses\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::UsedBy {
                    entity_name: entity.to_string(),
                }),
            ),
            
            // Calls - "what does X call"
            (
                Regex::new(r"(?i)what\s+(?:does|do)\s+(\w+)\s+call").unwrap(),
                Box::new(|entity: &str| QueryIntent::Calls {
                    entity_name: entity.to_string(),
                }),
            ),
            // Calls - "X calls what"
            (
                Regex::new(r"(?i)(\w+)\s+calls?\s+what").unwrap(),
                Box::new(|entity: &str| QueryIntent::Calls {
                    entity_name: entity.to_string(),
                }),
            ),
            // Called by - "what calls X"
            (
                Regex::new(r"(?i)what\s+calls\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::CalledBy {
                    entity_name: entity.to_string(),
                }),
            ),
            
            // Implements - "what does X implement"
            (
                Regex::new(r"(?i)what\s+(?:does|do)\s+(\w+)\s+implement").unwrap(),
                Box::new(|entity: &str| QueryIntent::Implements {
                    entity_name: entity.to_string(),
                }),
            ),
            // Implements - "X implements what"
            (
                Regex::new(r"(?i)(\w+)\s+implements?\s+what").unwrap(),
                Box::new(|entity: &str| QueryIntent::Implements {
                    entity_name: entity.to_string(),
                }),
            ),
            // Implemented by - "what implements X"
            (
                Regex::new(r"(?i)what\s+implements\s+(\w+)").unwrap(),
                Box::new(|entity: &str| QueryIntent::ImplementedBy {
                    entity_name: entity.to_string(),
                }),
            ),
            
            // All relationships - "what does X interact with"
            (
                Regex::new(r"(?i)what\s+(?:does|do)\s+(\w+)\s+interact").unwrap(),
                Box::new(|entity: &str| QueryIntent::AllRelationships {
                    entity_name: entity.to_string(),
                }),
            ),
            // All relationships - "X interacts with"
            (
                Regex::new(r"(?i)(\w+)\s+interact(?:s)?\s+with").unwrap(),
                Box::new(|entity: &str| QueryIntent::AllRelationships {
                    entity_name: entity.to_string(),
                }),
            ),
            // All relationships - "relationships of X"
            (
                Regex::new(r"(?i)(?:show\s+)?(?:all\s+)?relationships\s+(?:of|for)\s+(\w+)")
                    .unwrap(),
                Box::new(|entity: &str| QueryIntent::AllRelationships {
                    entity_name: entity.to_string(),
                }),
            ),
        ];

        Self { patterns }
    }

    /// Parse a natural language query to extract intent and entity name
    pub fn parse(&self, query: &str) -> Result<QueryIntent> {
        if query.trim().is_empty() {
            return Err(KgError::Query(
                "Query cannot be empty".to_string(),
            ));
        }

        for (pattern, intent_fn) in &self.patterns {
            if let Some(captures) = pattern.captures(query) {
                if let Some(entity_match) = captures.get(1) {
                    return Ok(intent_fn(entity_match.as_str()));
                }
            }
        }

        Err(KgError::Query(format!(
            "Could not parse query: '{}'\n\nSupported patterns:\n\
             - 'what does X depend on' / 'dependencies of X'\n\
             - 'what depends on X' / 'dependents of X'\n\
             - 'what does X use' / 'what uses X'\n\
             - 'what does X call' / 'what calls X'\n\
             - 'what does X implement' / 'what implements X'\n\
             - 'what does X interact with' / 'relationships of X'",
            query
        )))
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a relationship query against the knowledge graph
///
/// Handles entity resolution (with disambiguation), query execution,
/// and returns the requested relationships.
pub async fn execute_relationship_query<T: KgClient + ?Sized>(
    client: &T,
    repository_id: i32,
    query: &str,
) -> Result<RelationshipQueryResult> {
    // Parse the query to understand intent
    let parser = QueryParser::new();
    let intent = parser.parse(query)?;

    // Extract entity name from intent
    let entity_name = match &intent {
        QueryIntent::Dependencies { entity_name }
        | QueryIntent::Dependents { entity_name }
        | QueryIntent::AllRelationships { entity_name }
        | QueryIntent::Uses { entity_name }
        | QueryIntent::UsedBy { entity_name }
        | QueryIntent::Calls { entity_name }
        | QueryIntent::CalledBy { entity_name }
        | QueryIntent::Implements { entity_name }
        | QueryIntent::ImplementedBy { entity_name } => entity_name,
    };

    // Find entities matching the name
    let matcher = EntityMatcher::new();
    let matches = matcher
        .find_entities_by_name(client, repository_id, entity_name)
        .await?;

    if matches.is_empty() {
        return Err(KgError::NotFound(format!(
            "No entities found matching '{}'. Try a different name or check if the repository is indexed.",
            entity_name
        )));
    }

    // Check for ambiguous results
    if matches.len() > 1 && (matches[0].confidence - matches[1].confidence).abs() < 0.05 {
        return Ok(RelationshipQueryResult::Ambiguous {
            query: query.to_string(),
            candidates: matches.into_iter().map(|m| m.entity).collect(),
        });
    }

    // Use the best match
    let entity = matches.into_iter().next().unwrap().entity;

    // Execute the appropriate query based on intent
    let relationships = match intent {
        QueryIntent::Dependencies { .. } => {
            let rels = client.get_outgoing_relationships(&entity.id).await?;
            EntityRelationships {
                dependencies: rels.dependencies,
                ..Default::default()
            }
        }
        QueryIntent::Dependents { .. } => {
            let rels = client.get_incoming_relationships(&entity.id).await?;
            EntityRelationships {
                dependents: rels.dependents,
                ..Default::default()
            }
        }
        QueryIntent::Uses { .. } => {
            let rels = client.get_outgoing_relationships(&entity.id).await?;
            EntityRelationships {
                uses: rels.uses,
                ..Default::default()
            }
        }
        QueryIntent::UsedBy { .. } => {
            let rels = client.get_incoming_relationships(&entity.id).await?;
            EntityRelationships {
                used_by: rels.used_by,
                ..Default::default()
            }
        }
        QueryIntent::Calls { .. } => {
            let rels = client.get_outgoing_relationships(&entity.id).await?;
            EntityRelationships {
                calls: rels.calls,
                ..Default::default()
            }
        }
        QueryIntent::CalledBy { .. } => {
            let rels = client.get_incoming_relationships(&entity.id).await?;
            EntityRelationships {
                called_by: rels.called_by,
                ..Default::default()
            }
        }
        QueryIntent::Implements { .. } => {
            let rels = client.get_outgoing_relationships(&entity.id).await?;
            EntityRelationships {
                implements: rels.implements,
                ..Default::default()
            }
        }
        QueryIntent::ImplementedBy { .. } => {
            let rels = client.get_incoming_relationships(&entity.id).await?;
            EntityRelationships {
                implemented_by: rels.implemented_by,
                ..Default::default()
            }
        }
        QueryIntent::AllRelationships { .. } => client.get_all_relationships(&entity.id).await?,
    };

    Ok(RelationshipQueryResult::Success {
        entity,
        relationships: Box::new(relationships),
    })
}

/// Result of a relationship query
#[derive(Debug, Clone)]
pub enum RelationshipQueryResult {
    /// Query successful, returning entity and its relationships
    Success {
        entity: CodeEntity,
        relationships: Box<EntityRelationships>,
    },
    /// Multiple entities match the query, disambiguation needed
    Ambiguous {
        query: String,
        candidates: Vec<CodeEntity>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_parser_dependencies() {
        let parser = QueryParser::new();

        let intent = parser.parse("what does Parser depend on").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Dependencies {
                entity_name: "Parser".to_string()
            }
        );

        let intent = parser.parse("dependencies of Config").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Dependencies {
                entity_name: "Config".to_string()
            }
        );

        let intent = parser.parse("show dependencies for Database").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Dependencies {
                entity_name: "Database".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_dependents() {
        let parser = QueryParser::new();

        let intent = parser.parse("what depends on Parser").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Dependents {
                entity_name: "Parser".to_string()
            }
        );

        let intent = parser.parse("dependents of Config").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Dependents {
                entity_name: "Config".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_uses() {
        let parser = QueryParser::new();

        let intent = parser.parse("what does Parser use").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Uses {
                entity_name: "Parser".to_string()
            }
        );

        let intent = parser.parse("what uses Config").unwrap();
        assert_eq!(
            intent,
            QueryIntent::UsedBy {
                entity_name: "Config".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_calls() {
        let parser = QueryParser::new();

        let intent = parser.parse("what does main call").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Calls {
                entity_name: "main".to_string()
            }
        );

        let intent = parser.parse("what calls parse_file").unwrap();
        assert_eq!(
            intent,
            QueryIntent::CalledBy {
                entity_name: "parse_file".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_implements() {
        let parser = QueryParser::new();

        let intent = parser.parse("what does Database implement").unwrap();
        assert_eq!(
            intent,
            QueryIntent::Implements {
                entity_name: "Database".to_string()
            }
        );

        let intent = parser.parse("what implements Serializable").unwrap();
        assert_eq!(
            intent,
            QueryIntent::ImplementedBy {
                entity_name: "Serializable".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_all_relationships() {
        let parser = QueryParser::new();

        let intent = parser.parse("what does Parser interact with").unwrap();
        assert_eq!(
            intent,
            QueryIntent::AllRelationships {
                entity_name: "Parser".to_string()
            }
        );

        let intent = parser.parse("relationships of Config").unwrap();
        assert_eq!(
            intent,
            QueryIntent::AllRelationships {
                entity_name: "Config".to_string()
            }
        );

        let intent = parser.parse("show all relationships for Database").unwrap();
        assert_eq!(
            intent,
            QueryIntent::AllRelationships {
                entity_name: "Database".to_string()
            }
        );
    }

    #[test]
    fn test_query_parser_invalid() {
        let parser = QueryParser::new();

        let result = parser.parse("random nonsense query");
        assert!(result.is_err());
        assert!(matches!(result, Err(KgError::Query(_))));
    }

    #[test]
    fn test_query_parser_empty() {
        let parser = QueryParser::new();

        let result = parser.parse("");
        assert!(result.is_err());
        assert!(matches!(result, Err(KgError::Query(_))));
    }

    #[test]
    fn test_entity_matcher_similarity() {
        let matcher = EntityMatcher::new();

        // Exact match
        assert_eq!(matcher.compute_similarity("Parser", "Parser"), 1.0);

        // Case insensitive
        assert_eq!(matcher.compute_similarity("Parser", "parser"), 1.0);

        // Similar (typo)
        let sim = matcher.compute_similarity("Parser", "Parsr");
        assert!(sim > 0.85, "Expected similarity > 0.85, got {}", sim);

        // Different - Jaro-Winkler gives higher scores for prefix matches
        // "Parser" vs "Database" have no common prefix, so score should be lower
        let sim = matcher.compute_similarity("Parser", "Database");
        assert!(sim < 0.65, "Expected similarity < 0.65 for different words, got {}", sim);
    }

    #[test]
    fn test_entity_matcher_custom_threshold() {
        let matcher = EntityMatcher::with_threshold(0.5).unwrap();
        assert_eq!(matcher.threshold, 0.5);

        let result = EntityMatcher::with_threshold(1.5);
        assert!(result.is_err());
        assert!(matches!(result, Err(KgError::InvalidInput(_))));
    }
}
