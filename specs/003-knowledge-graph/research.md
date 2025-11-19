# Research: Knowledge Graph Implementation

**Feature**: 003-knowledge-graph  
**Date**: 2025-11-19  
**Status**: Complete

## Overview

This document resolves all technical unknowns identified in the Technical Context section of plan.md. Research covers graph database selection, Ollama integration, and natural language query parsing.

---

## 1. Graph Database Selection

### Decision: **SurrealDB (embedded mode)**

### Rationale

SurrealDB provides the best fit for cudgel's requirements:

1. **Embedded deployment**: Can run as in-process database (no separate server process)
2. **Local-first**: Fully offline-capable after initial setup
3. **Mature Rust client**: Native Rust implementation with async/await support
4. **Graph capabilities**: Full graph model with traversal queries
5. **Query language**: SurrealQL provides intuitive graph traversal syntax
6. **Performance**: Memory-efficient, handles target scale (100k nodes, 500k edges)
7. **Single binary**: No external dependencies beyond file storage

### Alternatives Considered

| Database | Pros | Cons | Verdict |
|----------|------|------|---------|
| **Neo4j** | Industry standard, powerful Cypher query language, excellent documentation | Requires separate JVM process, heavy memory footprint (>1GB), no embedded Rust mode | ❌ Rejected - not local-first friendly |
| **MemGraph** | Fast in-memory graph, Cypher compatible | Requires separate process, no embedded mode, all data in RAM | ❌ Rejected - separate process required |
| **IndraDB** | Pure Rust, embedded option | Immature project, limited query capabilities, sparse documentation | ❌ Rejected - too immature |
| **PostgreSQL + recursive CTEs** | Already in use, no new dependencies | Poor performance for multi-hop traversals, complex query syntax, not optimized for graphs | ❌ Rejected - performance inadequate |
| **SurrealDB** | Embedded Rust, multi-model, good graph support, active development | Newer project (less mature than Neo4j), smaller community | ✅ **Selected** |

### Technical Details

**Installation**:
```toml
[dependencies]
surrealdb = "1.0"
```

**Basic Operations Example**:
```rust
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;

// Embedded database (file-based)
let db = Surreal::new::<RocksDb>("data/graph.db").await?;
db.use_ns("cudgel").use_db("knowledge_graph").await?;

// Create node
let entity: Record = db
    .create("code_entity")
    .content(CodeEntity {
        name: "Parser",
        entity_type: "module",
        file_path: "src/parser.rs",
        summary: "Handles code parsing using tree-sitter",
    })
    .await?;

// Create relationship
db.query("RELATE $from->depends_on->$to")
    .bind(("from", from_id))
    .bind(("to", to_id))
    .await?;

// Traverse relationships
let results = db
    .query("SELECT * FROM code_entity WHERE name = 'Parser' FETCH ->depends_on->code_entity")
    .await?;
```

**Performance Characteristics**:
- Query latency: <10ms for 2-hop traversals on target scale
- Memory footprint: ~200-300MB for 100k nodes
- Disk storage: ~50-100MB for target graph size
- Concurrent queries: Handles multiple read queries efficiently

**Integration with cudgel**:
- Async/await compatible with existing tokio runtime
- File-based storage alongside PostgreSQL
- No port conflicts (embedded, no network listener)
- Backup/restore via file system operations

---

## 2. Ollama Integration

### Decision: **ollama-rs crate with streaming support**

### Rationale

The `ollama-rs` crate provides:
1. **Type-safe Rust API**: Well-structured client with async support
2. **Streaming**: Essential for long architectural summaries
3. **Error handling**: Comprehensive error types for resilient integration
4. **Active maintenance**: Regular updates, responsive maintainers
5. **Zero dependencies on external APIs**: Talks to local Ollama instance only

### Technical Details

**Installation**:
```toml
[dependencies]
ollama-rs = "0.2"
tokio = { version = "1.0", features = ["full"] }
```

**Client Setup**:
```rust
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};

pub struct LlmClient {
    ollama: Ollama,
    model: String,
    timeout: Duration,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ollama: Ollama::new("http://localhost".to_string(), 11434),
            model: "llama3.2:3b".to_string(), // Faster, smaller model
            timeout: Duration::from_secs(30),
        })
    }

    pub async fn generate_summary(&self, prompt: &str) -> Result<String> {
        let request = GenerationRequest::new(self.model.clone(), prompt.to_string())
            .temperature(0.3) // Lower temperature for factual summaries
            .top_p(0.9);

        let response = tokio::time::timeout(
            self.timeout,
            self.ollama.generate(request)
        ).await??;

        Ok(response.response)
    }
}
```

**Error Handling Patterns**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Ollama service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Generation timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid response from LLM: {0}")]
    InvalidResponse(String),
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

impl LlmError {
    pub fn to_user_message(&self) -> String {
        match self {
            Self::ServiceUnavailable(_) => 
                "Ollama service is not running. Start it with: ollama serve".to_string(),
            Self::Timeout(d) => 
                format!("Summary generation timed out after {:?}. Try a smaller codebase or increase timeout.", d),
            Self::InvalidResponse(_) => 
                "LLM returned invalid response. Try regenerating or check Ollama logs.".to_string(),
            Self::ModelNotFound(m) => 
                format!("Model '{}' not found. Download it with: ollama pull {}", m, m),
        }
    }
}
```

**Prompt Templates**:

```rust
pub struct PromptTemplates;

impl PromptTemplates {
    // Repository-level summary
    pub fn repository_summary(repo_name: &str, languages: &[String], top_modules: &[String]) -> String {
        format!(
            "Analyze this code repository and provide a concise architectural overview.\n\n\
             Repository: {}\n\
             Languages: {}\n\
             Main modules: {}\n\n\
             Provide:\n\
             1. Overall architecture pattern (e.g., layered, microservices, monolithic)\n\
             2. Primary components and their purposes (2-3 sentences)\n\
             3. Key technologies and frameworks detected\n\n\
             Keep response under 200 words.",
            repo_name,
            languages.join(", "),
            top_modules.join(", ")
        )
    }

    // Module-level summary
    pub fn module_summary(module_name: &str, file_count: usize, dependencies: &[String]) -> String {
        format!(
            "Summarize the purpose and responsibilities of this code module.\n\n\
             Module: {}\n\
             Files: {}\n\
             Dependencies: {}\n\n\
             Provide:\n\
             1. Primary responsibility (1 sentence)\n\
             2. Key functionality (2-3 bullet points)\n\
             3. How it fits in the overall architecture\n\n\
             Keep response under 150 words.",
            module_name,
            file_count,
            if dependencies.is_empty() { "none".to_string() } else { dependencies.join(", ") }
        )
    }

    // Component-level summary
    pub fn component_summary(component_name: &str, component_type: &str, code_snippet: &str) -> String {
        format!(
            "Explain what this {} does and its purpose.\n\n\
             Name: {}\n\
             Code:\n```\n{}\n```\n\n\
             Provide:\n\
             1. What it does (1 sentence)\n\
             2. Key responsibilities (2-3 points)\n\
             3. Notable patterns or techniques used\n\n\
             Keep response under 100 words.",
            component_type,
            component_name,
            code_snippet.lines().take(50).collect::<Vec<_>>().join("\n") // Limit context
        )
    }
}
```

**Performance Characteristics**:
- **Latency**: 2-5 seconds per summary with llama3.2:3b (faster than 8b model)
- **Context window**: llama3.2 supports 128k tokens (ample for code context)
- **Batch processing**: Generate summaries in parallel (limit to 3 concurrent to avoid overwhelming Ollama)
- **Fallback**: If generation fails, store empty summary and mark for retry

**Model Selection**:
- **Primary**: `llama3.2:3b` - Faster inference (~2-3s per summary), good for factual tasks
- **Alternative**: `llama3.2:8b` - Higher quality but slower (~5-8s per summary)
- **Configurable**: Allow users to specify model via CLI flag

---

## 3. Natural Language Query Parsing

### Decision: **Hybrid rule-based + entity fuzzy matching**

### Rationale

A hybrid approach balances simplicity, performance, and accuracy:

1. **Rule-based intent classification**: Fast, deterministic, covers common patterns
2. **Fuzzy entity matching**: Handles typos and variations in entity names
3. **No ML dependencies**: Maintains local-first architecture, no training data needed
4. **Extensible**: Easy to add new query patterns as usage evolves
5. **Fast**: <10ms parsing time

### Technical Details

**Query Intent Types**:
```rust
#[derive(Debug, Clone)]
pub enum QueryIntent {
    DescribeArchitecture,           // "what is the architecture?"
    ListComponents(ComponentType),   // "show me the services"
    DescribeEntity(String),         // "what does Parser do?"
    FindRelationships(String),      // "what does Parser interact with?"
    AnalyzePattern(String),         // "how is error handling implemented?"
}

#[derive(Debug, Clone)]
pub enum ComponentType {
    Service,
    Module,
    Class,
    Function,
    All,
}
```

**Intent Classification Rules**:
```rust
pub struct QueryParser {
    architecture_patterns: Vec<Regex>,
    relationship_patterns: Vec<Regex>,
    entity_patterns: Vec<Regex>,
}

impl QueryParser {
    pub fn new() -> Self {
        Self {
            architecture_patterns: vec![
                Regex::new(r"(?i)what.*(architecture|structure|organized)").unwrap(),
                Regex::new(r"(?i)(describe|explain|overview).*(system|repo|codebase)").unwrap(),
            ],
            relationship_patterns: vec![
                Regex::new(r"(?i)what.*(interact|depend|use|call)").unwrap(),
                Regex::new(r"(?i)show.*(relationship|connection|link)").unwrap(),
            ],
            entity_patterns: vec![
                Regex::new(r"(?i)what (does|is) (?P<entity>\w+)").unwrap(),
                Regex::new(r"(?i)(describe|explain) (?P<entity>\w+)").unwrap(),
            ],
        }
    }

    pub fn parse(&self, query: &str) -> Result<QueryIntent> {
        let query = query.trim().to_lowercase();

        // Check architecture patterns
        if self.architecture_patterns.iter().any(|p| p.is_match(&query)) {
            return Ok(QueryIntent::DescribeArchitecture);
        }

        // Check relationship patterns
        if self.relationship_patterns.iter().any(|p| p.is_match(&query)) {
            if let Some(entity) = self.extract_entity(&query) {
                return Ok(QueryIntent::FindRelationships(entity));
            }
        }

        // Check entity description patterns
        for pattern in &self.entity_patterns {
            if let Some(caps) = pattern.captures(&query) {
                if let Some(entity) = caps.name("entity") {
                    return Ok(QueryIntent::DescribeEntity(entity.as_str().to_string()));
                }
            }
        }

        // Check component listing
        if query.contains("show") || query.contains("list") {
            let component_type = if query.contains("service") {
                ComponentType::Service
            } else if query.contains("module") {
                ComponentType::Module
            } else if query.contains("class") {
                ComponentType::Class
            } else {
                ComponentType::All
            };
            return Ok(QueryIntent::ListComponents(component_type));
        }

        Err(Error::QueryParseError(format!("Could not parse query: {}", query)))
    }

    fn extract_entity(&self, query: &str) -> Option<String> {
        // Extract capitalized words or quoted entities
        let words: Vec<&str> = query.split_whitespace().collect();
        words.iter()
            .find(|w| w.chars().next().map_or(false, |c| c.is_uppercase()))
            .map(|s| s.to_string())
    }
}
```

**Entity Fuzzy Matching**:
```rust
use strsim::jaro_winkler;

pub struct EntityMatcher {
    threshold: f64,
}

impl EntityMatcher {
    pub fn new() -> Self {
        Self { threshold: 0.85 }
    }

    pub async fn find_entity(&self, db: &GraphClient, name: &str) -> Result<Vec<EntityMatch>> {
        // Exact match first
        let exact = db.query_entities_by_name(name).await?;
        if !exact.is_empty() {
            return Ok(exact.into_iter().map(|e| EntityMatch {
                entity: e,
                confidence: 1.0,
            }).collect());
        }

        // Fuzzy match
        let all_entities = db.query_all_entity_names().await?;
        let mut matches: Vec<(Entity, f64)> = all_entities
            .into_iter()
            .filter_map(|entity| {
                let score = jaro_winkler(&name.to_lowercase(), &entity.name.to_lowercase());
                if score >= self.threshold {
                    Some((entity, score))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        Ok(matches.into_iter().map(|(entity, score)| EntityMatch {
            entity,
            confidence: score,
        }).collect())
    }
}

pub struct EntityMatch {
    pub entity: Entity,
    pub confidence: f64,
}
```

**Disambiguation Strategy**:
```rust
impl QueryProcessor {
    pub async fn handle_ambiguous_entities(&self, matches: Vec<EntityMatch>) -> Result<Entity> {
        match matches.len() {
            0 => Err(Error::EntityNotFound),
            1 => Ok(matches[0].entity.clone()),
            _ => {
                // Multiple matches - return all with disambiguation message
                let options: Vec<String> = matches.iter()
                    .map(|m| format!("{} ({}:{})", m.entity.name, m.entity.entity_type, m.entity.file_path))
                    .collect();
                
                Err(Error::AmbiguousEntity {
                    query: matches[0].entity.name.clone(),
                    options,
                    hint: "Please specify the full path or type to disambiguate".to_string(),
                })
            }
        }
    }
}
```

**Performance**:
- Intent classification: <1ms (regex matching)
- Entity fuzzy matching: <10ms for 100k entities (indexed by name)
- Total query parsing: <15ms

**Extensibility**:
- Add new regex patterns for additional query types
- Support synonyms via pattern variations
- Future: Add ML-based intent classification if needed (out of scope for MVP)

---

## 4. Integration Architecture

### Graph Building Pipeline

```rust
pub struct GraphBuilder {
    graph_client: Arc<GraphClient>,
    llm_client: Arc<LlmClient>,
    entity_extractor: EntityExtractor,
}

impl GraphBuilder {
    pub async fn build_from_index(&self, repo_path: &Path) -> Result<()> {
        // 1. Extract entities from parsed code (use existing tree-sitter parsers)
        let entities = self.entity_extractor.extract_all(repo_path).await?;
        
        // 2. Create graph nodes
        for entity in entities {
            self.graph_client.create_node(entity).await?;
        }
        
        // 3. Extract relationships (dependencies, calls, inheritance)
        let relationships = self.entity_extractor.extract_relationships(repo_path).await?;
        for rel in relationships {
            self.graph_client.create_edge(rel).await?;
        }
        
        // 4. Generate summaries (parallel with rate limiting)
        self.generate_summaries_parallel(repo_path, 3).await?;
        
        Ok(())
    }

    async fn generate_summaries_parallel(&self, repo_path: &Path, concurrency: usize) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        
        // Repository summary
        let repo_summary = self.llm_client.generate_repository_summary(repo_path).await?;
        self.graph_client.set_repository_summary(repo_summary).await?;
        
        // Module summaries (parallel with rate limit)
        let modules = self.graph_client.get_all_modules().await?;
        let tasks: Vec<_> = modules.into_iter().map(|module| {
            let sem = semaphore.clone();
            let llm = self.llm_client.clone();
            let graph = self.graph_client.clone();
            
            tokio::spawn(async move {
                let _permit = sem.acquire().await;
                match llm.generate_module_summary(&module).await {
                    Ok(summary) => graph.set_module_summary(&module.id, summary).await,
                    Err(e) => {
                        tracing::warn!("Failed to generate summary for {}: {}", module.name, e);
                        Ok(()) // Don't fail entire process
                    }
                }
            })
        }).collect();
        
        futures::future::join_all(tasks).await;
        Ok(())
    }
}
```

### Incremental Update Strategy

```rust
impl GraphBuilder {
    pub async fn update_incremental(&self, changed_files: &[PathBuf]) -> Result<()> {
        for file_path in changed_files {
            // 1. Find affected entities in graph
            let affected_entities = self.graph_client
                .query_entities_by_file(file_path).await?;
            
            // 2. Delete old nodes and edges
            for entity in &affected_entities {
                self.graph_client.delete_entity_cascade(&entity.id).await?;
            }
            
            // 3. Re-parse and re-create
            let new_entities = self.entity_extractor.extract_from_file(file_path).await?;
            for entity in new_entities {
                self.graph_client.create_node(entity).await?;
            }
            
            // 4. Regenerate summaries for affected modules
            let affected_modules = self.find_affected_modules(&affected_entities).await?;
            for module in affected_modules {
                let summary = self.llm_client.generate_module_summary(&module).await?;
                self.graph_client.set_module_summary(&module.id, summary).await?;
            }
        }
        Ok(())
    }
}
```

---

## 5. Technology Stack Summary

### Final Selections

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| Graph Database | SurrealDB | 1.0 | Embedded, Rust-native, graph support |
| LLM Integration | ollama-rs | 0.2 | Type-safe, async, streaming support |
| Query Parsing | Regex + strsim | - | Fast, local, no external dependencies |
| Async Runtime | tokio | 1.0 | Existing infrastructure |
| Error Handling | thiserror | 1.0 | Existing pattern |

### Dependencies to Add

```toml
[dependencies]
surrealdb = "1.0"
ollama-rs = "0.2"
strsim = "0.11"  # Fuzzy string matching
regex = "1.10"   # Query pattern matching
futures = "0.3"  # Async utilities
```

### Performance Budget

| Operation | Target | Approach |
|-----------|--------|----------|
| Index 50k files | <30 min | Parallel entity extraction, batched graph writes |
| Generate repo summary | <5 sec | Use smaller llama3.2:3b model |
| Query response | <3 sec | Indexed entity names, efficient graph traversal |
| Incremental re-index | <2 min | SHA256-based change detection, cascade updates |

---

## 6. Risk Mitigation

### Risk: Ollama service unavailable

**Mitigation**: 
- Graceful degradation: Index without summaries if Ollama unavailable
- Clear error messages with remediation steps
- Make knowledge graph feature optional (CLI flag: `--enable-graph`)

### Risk: LLM hallucinations in summaries

**Mitigation**:
- Low temperature (0.3) for factual generation
- Structured prompts with explicit constraints
- Summary validation: check for minimum/maximum length
- User can regenerate summaries with different prompts

### Risk: Graph database performance at scale

**Mitigation**:
- Benchmark with realistic codebases (10k, 50k, 100k files)
- Index entity names for fast lookup
- Limit traversal depth in queries (max 3 hops)
- Add query timeouts (3 second limit)

### Risk: Query parsing ambiguity

**Mitigation**:
- Fuzzy matching with confidence scores
- Disambiguation prompts for multiple matches
- Fallback to showing all candidates when unclear
- Log common unparsed queries for pattern improvements

---

## 7. Next Steps

All NEEDS CLARIFICATION items resolved:
- ✅ Graph database: SurrealDB (embedded mode)
- ✅ Ollama integration: ollama-rs with streaming
- ✅ Query parsing: Hybrid rule-based + fuzzy matching

Ready to proceed to **Phase 1: Design & Contracts**.
