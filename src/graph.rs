//! Graph-based relationship queries

use crate::{database::Database, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Node in a code relationship graph
///
/// Represents a symbol in the call/reference graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Symbol database ID
    pub id: i32,
    /// Symbol name
    pub name: String,
    /// Symbol kind ("function", "class", etc.)
    pub kind: String,
    /// Source file path
    pub file: String,
    /// Programming language
    pub language: Option<String>,
    /// Line number in file (1-indexed)
    pub line: i32,
}

/// Edge in a code relationship graph
///
/// Represents a relationship between two symbols (call, import, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID
    pub from: i32,
    /// Target node ID
    pub to: i32,
    /// Edge type ("call", "import", "extends", etc.)
    #[serde(rename = "type")]
    pub edge_type: String,
    /// Line number where relationship occurs
    pub line: Option<i32>,
    /// Column number where relationship occurs
    pub column: Option<i32>,
}

/// Code relationship graph
///
/// Contains nodes (symbols) and edges (relationships) discovered by traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// All edges connecting nodes
    pub edges: Vec<GraphEdge>,
    /// Root symbol name that was queried
    pub root: String,
}

/// Graph query engine
///
/// Provides methods for traversing code relationships and building call graphs.
pub struct GraphQuery {
    db: Arc<Database>,
}

impl GraphQuery {
    /// Create a new graph query engine
    ///
    /// # Arguments
    /// * `db` - Database connection
    pub fn new(db: Arc<Database>) -> Self {
        GraphQuery { db }
    }

    /// Get references graph for a symbol
    ///
    /// Traverses the code relationship graph starting from the named symbol,
    /// following outgoing references up to the specified depth.
    ///
    /// # Arguments
    /// * `symbol_name` - Name of the root symbol to start traversal from
    /// * `repository_path` - Optional filter to specific repository
    /// * `depth` - Maximum traversal depth (number of hops from root)
    ///
    /// # Returns
    /// Graph containing all discovered nodes and edges
    pub async fn get_references(
        &self,
        symbol_name: &str,
        repository_path: Option<&str>,
        depth: usize,
    ) -> Result<Graph> {
        let symbol = self
            .db
            .get_symbol_by_name(symbol_name, repository_path)
            .await?;

        let Some(symbol) = symbol else {
            return Ok(Graph {
                nodes: vec![],
                edges: vec![],
                root: symbol_name.to_string(),
            });
        };

        let symbol_id: i32 = symbol.get("id");

        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();

        self.traverse_references(symbol_id, &mut nodes, &mut edges, depth, &mut visited)
            .await?;

        Ok(Graph {
            nodes: nodes.into_values().collect(),
            edges,
            root: symbol_name.to_string(),
        })
    }

    async fn traverse_references(
        &self,
        symbol_id: i32,
        nodes: &mut HashMap<i32, GraphNode>,
        edges: &mut Vec<GraphEdge>,
        depth: usize,
        visited: &mut HashSet<i32>,
    ) -> Result<()> {
        if depth == 0 || visited.contains(&symbol_id) {
            return Ok(());
        }

        visited.insert(symbol_id);

        // Get symbol information and add to nodes
        let symbol_row = self.db.get_symbol_by_id(symbol_id).await?;
        if let Some(row) = symbol_row {
            nodes.entry(symbol_id).or_insert_with(|| GraphNode {
                id: symbol_id,
                name: row.get("name"),
                kind: row.get("kind"),
                file: row.get("path"),
                language: row.get("language"),
                line: row.get("start_line"),
            });
        }

        let references = self.db.get_references(symbol_id).await?;

        for reference in references {
            edges.push(GraphEdge {
                from: reference.from_symbol_id,
                to: reference.to_symbol_id,
                edge_type: reference.reference_type,
                line: Some(reference.line),
                column: Some(reference.column),
            });

            // Get the target symbol and add to nodes
            let to_symbol_row = self.db.get_symbol_by_id(reference.to_symbol_id).await?;
            if let Some(row) = to_symbol_row {
                nodes
                    .entry(reference.to_symbol_id)
                    .or_insert_with(|| GraphNode {
                        id: reference.to_symbol_id,
                        name: row.get("name"),
                        kind: row.get("kind"),
                        file: row.get("path"),
                        language: row.get("language"),
                        line: row.get("start_line"),
                    });
            }

            // Recursively traverse
            Box::pin(self.traverse_references(
                reference.to_symbol_id,
                nodes,
                edges,
                depth - 1,
                visited,
            ))
            .await?;
        }

        Ok(())
    }

    /// Get call graph for a symbol
    ///
    /// Similar to `get_references` but filtered for function call relationships.
    /// Currently delegates to `get_references` with depth=2.
    ///
    /// # Arguments
    /// * `symbol_name` - Name of the root symbol
    /// * `repository_path` - Optional filter to specific repository
    /// * `_direction` - Direction of traversal (not yet implemented)
    ///
    /// # Returns
    /// Graph containing call relationships
    pub async fn get_call_graph(
        &self,
        symbol_name: &str,
        repository_path: Option<&str>,
        _direction: &str,
    ) -> Result<Graph> {
        // Similar to get_references but filtered for function calls
        self.get_references(symbol_name, repository_path, 2).await
    }
}
