//! `sam knowledge-graph` — a local, undirected knowledge graph where each node
//! carries a markdown note.
//!
//! Unlike the Supabase-backed commands, everything lives in a single JSON file
//! in the platform data directory, so the command works offline and never
//! prompts for auth. This module owns the data model and persistence; the
//! desktop app lives in [`super::knowledge_graph_gui`].

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
    // Load before opening the window so a corrupt-file error prints as a
    // normal error instead of flashing an empty app.
    let graph = load(&path)?;
    super::knowledge_graph_gui::run(graph, path)
}

/// Deterministic pseudo-random `[0, 1)` stream (an LCG); the layout must be
/// reproducible for a given seed so the map doesn't jiggle between frames.
fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as f64 / (1u64 << 31) as f64
}

/// Force-directed (Fruchterman–Reingold) layout with a light gravity term so
/// disconnected components stay on screen. Positions are index-aligned with
/// the node list; edges are index pairs. The result is deterministic in
/// `seed`, and "natural": linked nodes pull together, everything else pushes
/// apart.
pub fn force_layout(n: usize, edges: &[(usize, usize)], seed: u64) -> Vec<(f64, f64)> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![(0.0, 0.0)];
    }
    let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    // Start on a circle (deterministic, well-spread) with a little jitter so
    // symmetric graphs don't get stuck in an unstable equilibrium.
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / n as f64;
            (
                angle.cos() + 0.1 * (lcg(&mut rng) - 0.5),
                angle.sin() + 0.1 * (lcg(&mut rng) - 0.5),
            )
        })
        .collect();
    // Ideal edge length for a ~2×2 layout area.
    let k = (4.0 / n as f64).sqrt();
    let mut temperature: f64 = 0.4;
    for _ in 0..300 {
        let mut disp = vec![(0.0f64, 0.0f64); n];
        // Repulsion between every pair.
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
                let force = k * k / dist;
                disp[i].0 += dx / dist * force;
                disp[i].1 += dy / dist * force;
                disp[j].0 -= dx / dist * force;
                disp[j].1 -= dy / dist * force;
            }
        }
        // Attraction along edges.
        for &(a, b) in edges {
            let dx = pos[a].0 - pos[b].0;
            let dy = pos[a].1 - pos[b].1;
            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
            let force = dist * dist / k;
            disp[a].0 -= dx / dist * force;
            disp[a].1 -= dy / dist * force;
            disp[b].0 += dx / dist * force;
            disp[b].1 += dy / dist * force;
        }
        for i in 0..n {
            // Gravity toward the origin keeps disconnected components from
            // drifting apart forever.
            disp[i].0 -= pos[i].0 * 0.05;
            disp[i].1 -= pos[i].1 * 0.05;
            let (dx, dy) = disp[i];
            let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
            let step = dist.min(temperature);
            pos[i].0 += dx / dist * step;
            pos[i].1 += dy / dist * step;
        }
        temperature *= 0.98;
    }
    pos
}
