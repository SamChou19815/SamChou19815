//! `sam knowledge-graph` — a local, undirected knowledge graph where each node
//! carries a markdown note.
//!
//! Unlike the Supabase-backed commands, everything lives in a single JSON file
//! in the platform data directory, so the command works offline and never
//! prompts for auth. This module owns the data model and persistence; the
//! interactive TUI lives in [`super::knowledge_graph_tui`].

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub markdown: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    /// Monotonic id source. Ids are never reused, so an id in a stale edge or
    /// note can never silently point at a different node.
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Undirected edges, normalized as `(lo, hi)` id pairs with no duplicates.
    #[serde(default)]
    pub edges: Vec<(u64, u64)>,
}

/// Normalize an undirected edge to its canonical `(lo, hi)` form.
fn edge_key(a: u64, b: u64) -> (u64, u64) {
    (a.min(b), a.max(b))
}

impl Graph {
    pub fn add_node(&mut self, title: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(Node {
            id,
            title,
            markdown: String::new(),
        });
        id
    }

    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// Remove a node and every edge incident to it.
    pub fn remove_node(&mut self, id: u64) {
        self.nodes.retain(|node| node.id != id);
        self.edges.retain(|(a, b)| *a != id && *b != id);
    }

    /// Toggle the undirected edge between two nodes; returns whether the pair
    /// is linked afterwards. Self-loops are rejected (an undirected knowledge
    /// link from a note to itself is meaningless).
    pub fn toggle_edge(&mut self, a: u64, b: u64) -> bool {
        if a == b {
            return false;
        }
        let key = edge_key(a, b);
        if let Some(index) = self.edges.iter().position(|edge| *edge == key) {
            self.edges.remove(index);
            false
        } else {
            self.edges.push(key);
            true
        }
    }

    pub fn is_linked(&self, a: u64, b: u64) -> bool {
        self.edges.contains(&edge_key(a, b))
    }

    pub fn neighbors(&self, id: u64) -> Vec<u64> {
        self.edges
            .iter()
            .filter_map(|(a, b)| match () {
                () if *a == id => Some(*b),
                () if *b == id => Some(*a),
                () => None,
            })
            .collect()
    }

    pub fn degree(&self, id: u64) -> usize {
        self.neighbors(id).len()
    }
}

/// Where the graph is stored: `<platform data dir>/knowledge_graph.json`.
fn default_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "developersam", "sam-cli")
        .context("could not determine a data directory for this platform")?;
    Ok(dirs.data_dir().join("knowledge_graph.json"))
}

pub fn load(path: &Path) -> Result<Graph> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Graph::default()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing knowledge graph at {}", path.display()))
}

pub fn save(path: &Path, graph: &Graph) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(graph)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn run() -> Result<()> {
    let path = default_path()?;
    // Load before entering the alternate screen so a corrupt-file error prints
    // as a normal error instead of flashing an empty TUI.
    let graph = load(&path)?;
    super::knowledge_graph_tui::run(graph, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_assigns_fresh_ids_even_after_deletes() {
        let mut graph = Graph::default();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        graph.remove_node(a);
        let c = graph.add_node("c".to_string());
        assert_eq!((a, b, c), (0, 1, 2));
    }

    #[test]
    fn toggle_edge_is_undirected_and_normalized() {
        let mut graph = Graph::default();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        assert!(graph.toggle_edge(b, a));
        assert!(graph.is_linked(a, b));
        assert_eq!(graph.edges, vec![(a, b)]);
        // Toggling from the other direction removes the same edge.
        assert!(!graph.toggle_edge(a, b));
        assert!(graph.edges.is_empty());
        // Self-loops are rejected.
        assert!(!graph.toggle_edge(a, a));
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn remove_node_drops_incident_edges() {
        let mut graph = Graph::default();
        let a = graph.add_node("a".to_string());
        let b = graph.add_node("b".to_string());
        let c = graph.add_node("c".to_string());
        graph.toggle_edge(a, b);
        graph.toggle_edge(b, c);
        graph.toggle_edge(a, c);
        graph.remove_node(b);
        assert_eq!(graph.edges, vec![(a, c)]);
        assert_eq!(graph.neighbors(a), vec![c]);
        assert_eq!(graph.degree(c), 1);
    }

    #[test]
    fn graph_roundtrips_through_json() {
        let mut graph = Graph::default();
        let a = graph.add_node("Rust".to_string());
        let b = graph.add_node("TUI".to_string());
        graph.toggle_edge(a, b);
        graph.node_mut(a).unwrap().markdown = "# Notes\n".to_string();
        let json = serde_json::to_string(&graph).unwrap();
        let back: Graph = serde_json::from_str(&json).unwrap();
        assert_eq!(back.next_id, 2);
        assert_eq!(back.nodes.len(), 2);
        assert_eq!(back.edges, vec![(a, b)]);
        assert_eq!(back.nodes[0].markdown, "# Notes\n");
    }
}
