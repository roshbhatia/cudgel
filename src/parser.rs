//! Tree-sitter based code parsing

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

/// Abstract Syntax Tree node
///
/// Represents a node in the parsed AST with position and children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTNode {
    /// Node type from tree-sitter grammar (e.g., "function_definition")
    pub node_type: String,
    /// Source text span for this node
    pub text: String,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Starting column number (0-indexed)
    pub start_column: usize,
    /// Ending line number (0-indexed)
    pub end_line: usize,
    /// Ending column number (0-indexed)
    pub end_column: usize,
    /// Child AST nodes
    pub children: Vec<ASTNode>,
}

/// Extracted code symbol from parsing
///
/// Represents a function, class, method, or other symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind ("function", "class", "method", "struct", etc.)
    pub kind: String,
    /// Full signature (for functions/methods)
    pub signature: Option<String>,
    /// Documentation string
    pub docstring: Option<String>,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Ending line number (0-indexed)
    pub end_line: usize,
    /// Full source text
    pub text: String,
}

/// Multi-language code parser using tree-sitter
///
/// Maintains a cache of tree-sitter parsers for different languages.
pub struct CodeParser {
    parsers: HashMap<String, Parser>,
}

impl CodeParser {
    /// Create a new code parser
    pub fn new() -> Self {
        CodeParser {
            parsers: HashMap::new(),
        }
    }

    /// Detect programming language from file extension
    ///
    /// # Arguments
    /// * `path` - File path to analyze
    ///
    /// # Returns
    /// Language name if recognized, None otherwise
    pub fn detect_language(path: &Path) -> Option<String> {
        let ext = path.extension()?.to_str()?;

        match ext {
            "py" | "pyw" => Some("python".to_string()),
            "js" | "jsx" | "mjs" => Some("javascript".to_string()),
            "ts" | "tsx" => Some("typescript".to_string()),
            "rs" => Some("rust".to_string()),
            "go" => Some("go".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some("cpp".to_string()),
            "java" => Some("java".to_string()),
            _ => None,
        }
    }

    fn get_language(lang: &str) -> Result<Language> {
        match lang {
            "python" => Ok(tree_sitter_python::language()),
            "javascript" => Ok(tree_sitter_javascript::language()),
            "typescript" => Ok(tree_sitter_typescript::language_typescript()),
            "rust" => Ok(tree_sitter_rust::language()),
            "go" => Ok(tree_sitter_go::language()),
            "c" => Ok(tree_sitter_c::language()),
            "cpp" => Ok(tree_sitter_cpp::language()),
            "java" => Ok(tree_sitter_java::language()),
            _ => Err(Error::UnsupportedLanguage(lang.to_string())),
        }
    }

    fn get_parser(&mut self, language: &str) -> Result<&mut Parser> {
        if !self.parsers.contains_key(language) {
            let mut parser = Parser::new();
            let lang = Self::get_language(language)?;
            parser
                .set_language(&lang)
                .map_err(|e| Error::Parse(e.to_string()))?;
            self.parsers.insert(language.to_string(), parser);
        }

        self.parsers.get_mut(language).ok_or_else(|| {
            Error::Parse(format!(
                "Parser for language '{}' not found after initialization",
                language
            ))
        })
    }

    /// Parse a source file into an AST
    ///
    /// Detects language from file extension, parses using tree-sitter,
    /// and returns the AST tree with detected language.
    ///
    /// # Arguments
    /// * `path` - File path (used for language detection)
    /// * `content` - File content to parse
    ///
    /// # Returns
    /// Tuple of (AST root node, detected language name)
    pub fn parse_file(&mut self, path: &Path, content: &str) -> Result<(ASTNode, String)> {
        let language = Self::detect_language(path)
            .ok_or_else(|| Error::Parse(format!("Cannot detect language for {:?}", path)))?;

        let parser = self.get_parser(&language)?;
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| Error::Parse("Failed to parse file".to_string()))?;

        let root = self.convert_node(tree.root_node(), content);
        let hash = self.compute_hash(content);

        Ok((root, hash))
    }

    fn convert_node(&self, node: Node, source: &str) -> ASTNode {
        Self::convert_node_static(node, source)
    }

    fn convert_node_static(node: Node, source: &str) -> ASTNode {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");

        let children: Vec<ASTNode> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .map(|child| Self::convert_node_static(child, source))
            .collect();

        ASTNode {
            node_type: node.kind().to_string(),
            text: text.to_string(),
            start_line: node.start_position().row,
            start_column: node.start_position().column,
            end_line: node.end_position().row,
            end_column: node.end_position().column,
            children,
        }
    }

    /// Extract symbols (functions, classes, etc.) from AST
    ///
    /// Recursively traverses the AST and extracts all code symbols
    /// based on language-specific node types.
    ///
    /// # Arguments
    /// * `ast` - AST root node to traverse
    /// * `language` - Language name (affects which node types are recognized as symbols)
    ///
    /// # Returns
    /// Vector of extracted symbols with their metadata
    pub fn extract_symbols(&self, ast: &ASTNode, language: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        self.extract_symbols_recursive(ast, language, &mut symbols);
        symbols
    }

    fn extract_symbols_recursive(&self, node: &ASTNode, language: &str, symbols: &mut Vec<Symbol>) {
        let symbol_types = match language {
            "python" => vec!["function_definition", "class_definition"],
            "javascript" | "typescript" => vec![
                "function_declaration",
                "arrow_function",
                "class_declaration",
            ],
            "rust" => vec!["function_item", "struct_item", "enum_item", "trait_item"],
            "go" => vec![
                "function_declaration",
                "method_declaration",
                "type_declaration",
            ],
            "c" | "cpp" => vec!["function_definition", "class_specifier", "struct_specifier"],
            "java" => vec!["method_declaration", "class_declaration"],
            _ => vec![],
        };

        if symbol_types.contains(&node.node_type.as_str()) {
            if let Some(symbol) = self.create_symbol(node, language) {
                symbols.push(symbol);
            }
        }

        for child in &node.children {
            self.extract_symbols_recursive(child, language, symbols);
        }
    }

    fn create_symbol(&self, node: &ASTNode, language: &str) -> Option<Symbol> {
        let name = self.extract_symbol_name(node)?;
        let kind = self.get_symbol_kind(&node.node_type, language);
        let signature = self.extract_signature(node);
        let docstring = self.extract_docstring(node, language);

        Some(Symbol {
            name,
            kind,
            signature,
            docstring,
            start_line: node.start_line,
            end_line: node.end_line,
            text: node.text.clone(),
        })
    }

    fn extract_symbol_name(&self, node: &ASTNode) -> Option<String> {
        for child in &node.children {
            if child.node_type.contains("identifier") || child.node_type.contains("name") {
                return Some(child.text.clone());
            }
        }
        None
    }

    fn get_symbol_kind(&self, node_type: &str, _language: &str) -> String {
        match node_type {
            t if t.contains("function") => "function".to_string(),
            t if t.contains("class") => "class".to_string(),
            t if t.contains("method") => "method".to_string(),
            t if t.contains("struct") => "struct".to_string(),
            t if t.contains("enum") => "enum".to_string(),
            t if t.contains("trait") => "trait".to_string(),
            _ => "symbol".to_string(),
        }
    }

    fn extract_signature(&self, node: &ASTNode) -> Option<String> {
        let lines: Vec<&str> = node.text.lines().collect();
        if !lines.is_empty() {
            Some(lines[0].trim().to_string())
        } else {
            None
        }
    }

    fn extract_docstring(&self, node: &ASTNode, language: &str) -> Option<String> {
        if language == "python" {
            for child in &node.children {
                if child.node_type == "block" {
                    for stmt in &child.children {
                        if stmt.node_type.contains("string") {
                            let text = stmt.text.trim();
                            return Some(
                                text.trim_matches('"').trim_matches('\'').trim().to_string(),
                            );
                        }
                    }
                }
            }
        }
        None
    }

    fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }
}

impl Default for CodeParser {
    fn default() -> Self {
        Self::new()
    }
}
