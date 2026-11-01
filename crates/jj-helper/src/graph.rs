//! Lane-based layout of a [`Dag`] into text lines with box-drawing edges,
//! in the spirit of `jj log`'s graph. Each commit gets a node line and a
//! detail line; connector lines are inserted where lanes merge or fork.

use crate::dag::{Dag, Node, Preview};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Node,
    Detail,
    Connector,
}

#[derive(Clone, Debug)]
pub struct Line {
    /// Node this line belongs to.
    pub owner: String,
    pub kind: Kind,
    pub graph: String,
    /// Char index of the node glyph within `graph` (node lines only).
    pub node_at: Option<usize>,
}

#[derive(Clone, Copy, Default)]
struct Dirs {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    dashed: bool,
}

struct Grid {
    cells: Vec<Dirs>,
    fill: Vec<bool>,
    node: Option<(usize, char)>,
    tilde: Option<usize>,
}

impl Grid {
    fn new(width: usize) -> Grid {
        Grid {
            cells: vec![Dirs::default(); width],
            fill: vec![false; width],
            node: None,
            tilde: None,
        }
    }

    fn ensure(&mut self, i: usize) {
        if self.cells.len() <= i {
            self.cells.resize(i + 1, Dirs::default());
            self.fill.resize(i + 1, false);
        }
    }

    fn vert(&mut self, i: usize, dashed: bool) {
        self.ensure(i);
        let c = &mut self.cells[i];
        c.up = true;
        c.down = true;
        c.dashed |= dashed;
    }

    /// Horizontal run between cells `a` and `b` (exclusive of endpoints'
    /// vertical parts).
    fn horizontal(&mut self, a: usize, b: usize) {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        self.ensure(hi);
        self.cells[lo].right = true;
        self.cells[hi].left = true;
        for i in lo + 1..hi {
            self.cells[i].left = true;
            self.cells[i].right = true;
        }
        for f in &mut self.fill[lo..hi] {
            *f = true;
        }
    }

    /// Lane `j` comes down and merges sideways into lane `col`.
    fn merge(&mut self, j: usize, col: usize) {
        self.ensure(j.max(col));
        self.cells[j].up = true;
        self.horizontal(j, col);
    }

    /// Lane `col` forks sideways into a new lane `k` going down.
    fn fork(&mut self, col: usize, k: usize) {
        self.ensure(k.max(col));
        self.cells[k].down = true;
        self.horizontal(col, k);
    }

    fn render(&self) -> (String, Option<usize>) {
        let mut s = String::new();
        let mut node_at = None;
        for (i, d) in self.cells.iter().enumerate() {
            if let Some((n, ch)) = self.node {
                if n == i {
                    node_at = Some(s.chars().count());
                    s.push(ch);
                    s.push(if self.fill[i] { '─' } else { ' ' });
                    continue;
                }
            }
            if self.tilde == Some(i) {
                s.push('~');
            } else {
                s.push(glyph(d));
            }
            s.push(if self.fill[i] { '─' } else { ' ' });
        }
        (s, node_at)
    }
}

fn glyph(d: &Dirs) -> char {
    match (d.up, d.down, d.left, d.right) {
        (true, true, false, false) => {
            if d.dashed {
                '╎'
            } else {
                '│'
            }
        }
        (false, false, true, true) => '─',
        (true, false, true, false) => '╯',
        (true, false, false, true) => '╰',
        (false, true, true, false) => '╮',
        (false, true, false, true) => '╭',
        (true, true, false, true) => '├',
        (true, true, true, false) => '┤',
        (true, false, true, true) => '┴',
        (false, true, true, true) => '┬',
        (true, true, true, true) => '┼',
        (true, false, false, false) | (false, true, false, false) => '│',
        _ => ' ',
    }
}

pub fn node_glyph(n: &Node) -> char {
    match n.preview {
        Some(Preview::Ghost) | Some(Preview::SquashSource) => '◌',
        Some(Preview::Hidden) => '⊘',
        _ if n.commit.is_wc => '@',
        Some(Preview::New) => '+',
        _ if n.commit.conflict => '×',
        _ if n.commit.immutable => '◆',
        _ => '○',
    }
}

type Lane = Option<(String, bool)>;

fn occupied(lanes: &[Lane]) -> impl Iterator<Item = (usize, bool)> + '_ {
    lanes
        .iter()
        .enumerate()
        .filter_map(|(i, l)| l.as_ref().map(|(_, e)| (i, *e)))
}

pub fn layout(dag: &Dag) -> Vec<Line> {
    let mut lines = vec![];
    let mut lanes: Vec<Lane> = vec![];
    for id in &dag.order {
        let node = &dag.nodes[id];
        let targeting: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| l.as_ref().is_some_and(|(t, _)| t == id))
            .map(|(i, _)| i)
            .collect();
        let col = match targeting.first() {
            Some(&c) => c,
            None => match lanes.iter().position(|l| l.is_none()) {
                Some(free) => free,
                None => {
                    lanes.push(None);
                    lanes.len() - 1
                }
            },
        };
        let col_dashed = lanes[col].as_ref().is_some_and(|(_, e)| *e);

        if targeting.len() > 1 {
            let mut g = Grid::new(lanes.len());
            for (i, dashed) in occupied(&lanes) {
                if !targeting[1..].contains(&i) {
                    g.vert(i, dashed);
                }
            }
            for &j in &targeting[1..] {
                g.merge(j, col);
                lanes[j] = None;
            }
            g.vert(col, col_dashed);
            let (graph, _) = g.render();
            lines.push(Line {
                owner: id.clone(),
                kind: Kind::Connector,
                graph,
                node_at: None,
            });
            while lanes.len() > col + 1 && lanes.last().is_some_and(|l| l.is_none()) {
                lanes.pop();
            }
        }

        let mut g = Grid::new(lanes.len());
        for (i, dashed) in occupied(&lanes) {
            if i != col {
                g.vert(i, dashed);
            }
        }
        g.node = Some((col, node_glyph(node)));
        let (graph, node_at) = g.render();
        lines.push(Line {
            owner: id.clone(),
            kind: Kind::Node,
            graph,
            node_at,
        });

        lanes[col] = node.parents.first().cloned();
        let mut forks = vec![];
        // Extra parents that another lane is already heading to join that
        // lane directly instead of opening a parallel one.
        let mut joins = vec![];
        for p in node.parents.iter().skip(1) {
            if let Some(k) = lanes
                .iter()
                .enumerate()
                .position(|(i, l)| i != col && l.as_ref().is_some_and(|(t, _)| *t == p.0))
            {
                joins.push(k);
                continue;
            }
            let k = match lanes
                .iter()
                .enumerate()
                .position(|(i, l)| i > col && l.is_none())
            {
                Some(k) => k,
                None => {
                    lanes.push(None);
                    lanes.len() - 1
                }
            };
            lanes[k] = Some(p.clone());
            forks.push(k);
        }

        let mut g = Grid::new(lanes.len());
        for (i, dashed) in occupied(&lanes) {
            if !forks.contains(&i) {
                g.vert(i, dashed);
            }
        }
        let show_tilde = node.parents.is_empty() && node.truncated;
        if show_tilde {
            g.ensure(col);
            g.tilde = Some(col);
        }
        let (graph, _) = g.render();
        lines.push(Line {
            owner: id.clone(),
            kind: Kind::Detail,
            graph,
            node_at: None,
        });

        if !forks.is_empty() || !joins.is_empty() {
            let mut g = Grid::new(lanes.len());
            for (i, dashed) in occupied(&lanes) {
                if !forks.contains(&i) {
                    g.vert(i, dashed);
                }
            }
            for &k in forks.iter().chain(&joins) {
                g.fork(col, k);
            }
            let (graph, _) = g.render();
            lines.push(Line {
                owner: id.clone(),
                kind: Kind::Connector,
                graph,
                node_at: None,
            });
        }

        while lanes.last().is_some_and(|l| l.is_none()) {
            lanes.pop();
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::tests::graph;

    fn render(d: &Dag) -> String {
        layout(d)
            .iter()
            .map(|l| {
                let text = match l.kind {
                    Kind::Node => l.owner.clone(),
                    _ => String::new(),
                };
                format!("{}{}", l.graph, text).trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    #[test]
    fn linear_and_fork() {
        let d = graph(&[
            ("@c", &["b"]),
            ("b", &["a"]),
            ("x", &["T"]),
            ("a", &["T"]),
            ("T", &[]),
        ]);
        let out = render(&d);
        assert_eq!(
            out, "@ c\n│\n○ b\n│\n○ a\n│\n│ ○ x\n│ │\n├─╯\n◆ T",
            "\n{out}"
        );
    }

    #[test]
    fn merge_commit_forks_lanes() {
        let d = graph(&[("m", &["a", "b"]), ("a", &["T"]), ("b", &["T"]), ("T", &[])]);
        let out = render(&d);
        assert!(out.contains("├─╮"), "\n{out}");
        assert!(out.contains("├─╯"), "\n{out}");
    }

    #[test]
    fn preview_layout_is_consistent() {
        let mut d = graph(&[
            ("@c", &["b"]),
            ("b", &["a"]),
            ("x", &["T"]),
            ("a", &["T"]),
            ("T", &[]),
        ]);
        d.rebase_source("b", "x", true).unwrap();
        let lines = layout(&d);
        let nodes: Vec<_> = lines
            .iter()
            .filter(|l| l.kind == Kind::Node)
            .map(|l| l.owner.as_str())
            .collect();
        assert_eq!(nodes.len(), d.nodes.len());
        // Every node line has a glyph.
        assert!(lines
            .iter()
            .filter(|l| l.kind == Kind::Node)
            .all(|l| l.node_at.is_some()));
    }
}
