//! Graph-based relationship queries

use crate::{database::Database, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: i32,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub language: Option<String>,
    pub line: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: i32,
    pub to: i32,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub line: Option<i32>,
    pub column: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub root: String,
}

pub struct GraphQuery {
    db: Arc<Database>,
}

impl GraphQuery {
    pub fn new(db: Arc<Database>) -> Self {
        GraphQuery { db }
    }

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

        if symbol.is_none() {
            return Ok(Graph {
                nodes: vec![],
                edges: vec![],
                root: symbol_name.to_string(),
            });
        }

        let symbol = symbol.unwrap();
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
