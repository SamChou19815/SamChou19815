//! In-memory commit graph plus the pure transforms used to preview what an
//! operation will do before running it.
//!
//! This mirrors VisualJJ's (Sapling ISL's) `previewDag`: every operation can
//! rewrite a copy of the graph, tagging nodes with a [`Preview`] kind so the
//! renderer can show where things move, what disappears and what gets created.
//! Transforms return `Err(reason)` when the operation is invalid for the given
//! commits (e.g. rebasing onto a descendant, rewriting an immutable commit).

use std::collections::{HashMap, HashSet};

use crate::jj::{Commit, RepoGraph};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preview {
    /// Root of a set of commits that moves to a new parent.
    Moved,
    /// Carried along with a moved root.
    MovedDescendant,
    /// Where a moved commit used to be.
    Ghost,
    /// Will be abandoned.
    Hidden,
    /// Content folded into another commit (its old position).
    SquashSource,
    /// Receives squashed content.
    SquashTarget,
    /// Will be created.
    New,
    /// The working copy moves here.
    WcDestination,
    /// The working copy leaves this commit.
    WcPrevious,
    /// Description or content rewritten in place.
    Rewritten,
    /// Gets a PR created/updated.
    Submit,
    /// Will be merged into trunk.
    Ship,
    /// Gains a bookmark.
    BookmarkNew,
    /// Loses a bookmark.
    BookmarkOld,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub commit: Commit,
    /// `(parent id, elided)`; elided edges skip hidden commits.
    pub parents: Vec<(String, bool)>,
    /// Ancestry continues below the displayed window.
    pub truncated: bool,
    pub preview: Option<Preview>,
    pub note: Option<String>,
    /// Vertical placement hint: lower sorts higher on screen.
    pub sort_key: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Dag {
    pub nodes: HashMap<String, Node>,
    /// Display order, children before parents.
    pub order: Vec<String>,
}

pub type TResult = Result<(), String>;

pub const NEW_WC_ID: &str = "new:@";

impl Dag {
    pub fn from_repo(repo: &RepoGraph) -> Dag {
        let mut nodes = HashMap::new();
        let mut order = vec![];
        for (i, c) in repo.commits.iter().enumerate() {
            let edges = repo.edges.get(&c.commit_id).cloned().unwrap_or_default();
            nodes.insert(
                c.commit_id.clone(),
                Node {
                    id: c.commit_id.clone(),
                    commit: c.clone(),
                    parents: edges.parents,
                    truncated: edges.truncated,
                    preview: None,
                    note: None,
                    sort_key: i as f64,
                },
            );
            order.push(c.commit_id.clone());
        }
        let mut dag = Dag { nodes, order };
        // `jj log --no-graph` order isn't jj's graph order; re-sort so
        // branches stay contiguous.
        dag.resort();
        dag
    }

    pub fn get(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    fn node(&self, id: &str) -> Result<&Node, String> {
        self.nodes
            .get(id)
            .ok_or_else(|| format!("commit {id} is not in the graph"))
    }

    fn node_mut(&mut self, id: &str) -> &mut Node {
        self.nodes.get_mut(id).expect("node exists")
    }

    pub fn short(&self, id: &str) -> String {
        self.nodes
            .get(id)
            .map(|n| n.commit.short_change.clone())
            .unwrap_or_else(|| id.chars().take(8).collect())
    }

    pub fn wc(&self) -> Option<String> {
        self.order
            .iter()
            .find(|id| self.nodes[*id].commit.is_wc)
            .cloned()
    }

    pub fn trunk(&self) -> Option<String> {
        self.order
            .iter()
            .find(|id| self.nodes[*id].commit.trunk)
            .cloned()
    }

    pub fn parent_ids(&self, id: &str) -> Vec<String> {
        self.nodes
            .get(id)
            .map(|n| n.parents.iter().map(|(p, _)| p.clone()).collect())
            .unwrap_or_default()
    }

    /// Children in display order.
    pub fn children(&self, id: &str) -> Vec<String> {
        self.order
            .iter()
            .filter(|c| self.nodes[*c].parents.iter().any(|(p, _)| p == id))
            .cloned()
            .collect()
    }

    /// `id` and everything descending from it.
    pub fn descendants(&self, id: &str) -> HashSet<String> {
        let mut children_of: HashMap<&str, Vec<&str>> = HashMap::new();
        for n in self.nodes.values() {
            for (p, _) in &n.parents {
                children_of
                    .entry(p.as_str())
                    .or_default()
                    .push(n.id.as_str());
            }
        }
        let mut out = HashSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(cur) = stack.pop() {
            if out.insert(cur.clone()) {
                if let Some(cs) = children_of.get(cur.as_str()) {
                    stack.extend(cs.iter().map(|c| c.to_string()));
                }
            }
        }
        out
    }

    /// `id` and everything it descends from (within the graph).
    pub fn ancestors(&self, id: &str) -> HashSet<String> {
        let mut out = HashSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(cur) = stack.pop() {
            if self.nodes.contains_key(&cur) && out.insert(cur.clone()) {
                stack.extend(self.parent_ids(&cur));
            }
        }
        out
    }

    fn check_mutable<'a>(&self, ids: impl IntoIterator<Item = &'a String>) -> TResult {
        for id in ids {
            if let Some(n) = self.nodes.get(id) {
                if n.commit.immutable {
                    return Err(format!("{} is immutable", n.commit.short_change));
                }
            }
        }
        Ok(())
    }

    fn mark(&mut self, id: &str, preview: Preview, note: impl Into<Option<String>>) {
        if let Some(n) = self.nodes.get_mut(id) {
            n.preview = Some(preview);
            n.note = note.into();
        }
    }

    /// Replaces `old` among `child`'s parents with `new_parents`, keeping
    /// order and dropping duplicates.
    fn replace_parent(&mut self, child: &str, old: &str, new_parents: &[(String, bool)]) {
        let n = self.node_mut(child);
        let mut out: Vec<(String, bool)> = vec![];
        for (p, elided) in n.parents.drain(..) {
            if p == old {
                for np in new_parents {
                    if !out.iter().any(|(x, _)| x == &np.0) {
                        out.push(np.clone());
                    }
                }
            } else if !out.iter().any(|(x, _)| x == &p) {
                out.push((p, elided));
            }
        }
        n.parents = out;
    }

    /// Detaches `id` from its children, reparenting them onto its parents
    /// (what jj does to descendants of a moved or abandoned commit).
    fn splice_out(&mut self, id: &str) {
        let parents = self.nodes[id].parents.clone();
        for c in self.children(id) {
            self.replace_parent(&c, id, &parents);
        }
    }

    /// Places `ids` just above `anchor` in the vertical order.
    fn rekey(&mut self, ids: &HashSet<String>, anchor: &str) {
        let base = self.nodes[anchor].sort_key;
        let mut sorted: Vec<String> = ids.iter().cloned().collect();
        sorted.sort_by(|a, b| self.nodes[a].sort_key.total_cmp(&self.nodes[b].sort_key));
        let len = sorted.len() as f64;
        for (i, id) in sorted.iter().enumerate() {
            self.node_mut(id).sort_key = base - 0.5 * (len - i as f64) / (len + 1.0);
        }
    }

    fn add_ghosts(&mut self, ids: &HashSet<String>, original: &Dag, preview: Preview, note: &str) {
        for id in ids {
            let orig = &original.nodes[id];
            let ghost_id = format!("ghost:{id}");
            let parents = orig
                .parents
                .iter()
                .map(|(p, e)| {
                    (
                        if ids.contains(p) {
                            format!("ghost:{p}")
                        } else {
                            p.clone()
                        },
                        *e,
                    )
                })
                .collect();
            self.nodes.insert(
                ghost_id.clone(),
                Node {
                    id: ghost_id,
                    commit: Commit {
                        is_wc: false,
                        ..orig.commit.clone()
                    },
                    parents,
                    truncated: orig.truncated,
                    preview: Some(preview),
                    note: Some(note.to_string()),
                    sort_key: orig.sort_key,
                },
            );
        }
    }

    fn add_new(
        &mut self,
        id: &str,
        commit: Commit,
        parents: Vec<(String, bool)>,
        key: f64,
        note: &str,
    ) {
        self.nodes.insert(
            id.to_string(),
            Node {
                id: id.to_string(),
                commit,
                parents,
                truncated: false,
                preview: Some(Preview::New),
                note: Some(note.to_string()),
                sort_key: key,
            },
        );
    }

    /// The working copy leaves its current commit. jj abandons it when it is
    /// "discardable" (empty, undescribed, no children, no bookmarks).
    fn leave_wc(&mut self) {
        let Some(wc) = self.wc() else { return };
        let discardable = {
            let n = &self.nodes[&wc];
            n.commit.empty
                && n.commit.description.trim().is_empty()
                && n.commit.bookmarks.is_empty()
                && !n.commit.immutable
        } && self.children(&wc).is_empty();
        self.node_mut(&wc).commit.is_wc = false;
        if discardable {
            self.mark(
                &wc,
                Preview::Hidden,
                "abandoned (empty working copy)".to_string(),
            );
        } else {
            self.mark(&wc, Preview::WcPrevious, "@ leaves".to_string());
        }
    }

    fn new_wc_commit(&self, parents: &[(String, bool)]) -> Commit {
        let mut c = Commit::synthetic(NEW_WC_ID, "");
        c.is_wc = true;
        c.parents = parents.iter().map(|(p, _)| p.clone()).collect();
        c
    }

    // ---- operations -------------------------------------------------------

    /// `jj rebase -s src -o dest`
    pub fn rebase_source(&mut self, src: &str, dest: &str, ghosts: bool) -> TResult {
        self.node(src)?;
        self.node(dest)?;
        let moving = self.descendants(src);
        if moving.contains(dest) {
            return Err("can't rebase onto itself or a descendant".into());
        }
        self.check_mutable(&moving)?;
        if self.parent_ids(src) == [dest.to_string()] {
            return Err(format!(
                "{} is already on {}",
                self.short(src),
                self.short(dest)
            ));
        }
        let original = self.clone();
        self.node_mut(src).parents = vec![(dest.to_string(), false)];
        for id in &moving {
            self.mark(id, Preview::MovedDescendant, None);
        }
        let note = format!("rebased onto {}", self.short(dest));
        self.mark(src, Preview::Moved, note);
        self.rekey(&moving, dest);
        if ghosts {
            self.add_ghosts(&moving, &original, Preview::Ghost, "old position");
        }
        self.resort();
        Ok(())
    }

    /// `jj rebase -r rev -o dest`: only `rev` moves; its children stay put.
    pub fn rebase_revision(&mut self, rev: &str, dest: &str, ghosts: bool) -> TResult {
        self.node(rev)?;
        self.node(dest)?;
        if rev == dest {
            return Err("can't rebase a commit onto itself".into());
        }
        self.check_mutable(&self.descendants(rev))?;
        if self.parent_ids(rev) == [dest.to_string()] {
            return Err(format!(
                "{} is already on {}",
                self.short(rev),
                self.short(dest)
            ));
        }
        let original = self.clone();
        self.splice_out(rev);
        self.node_mut(rev).parents = vec![(dest.to_string(), false)];
        let note = format!("moved onto {}", self.short(dest));
        self.mark(rev, Preview::Moved, note);
        let set = HashSet::from([rev.to_string()]);
        self.rekey(&set, dest);
        if ghosts {
            self.add_ghosts(&set, &original, Preview::Ghost, "old position");
        }
        self.resort();
        Ok(())
    }

    /// `jj rebase -b rev -o dest`: the whole branch (relative to `dest`).
    pub fn rebase_branch(&mut self, rev: &str, dest: &str, ghosts: bool) -> TResult {
        self.node(rev)?;
        self.node(dest)?;
        let dest_anc = self.ancestors(dest);
        let branch: HashSet<String> = self.ancestors(rev).difference(&dest_anc).cloned().collect();
        if branch.is_empty() {
            return Err(format!(
                "{} is already based on {}",
                self.short(rev),
                self.short(dest)
            ));
        }
        let roots: Vec<String> = branch
            .iter()
            .filter(|id| !self.parent_ids(id).iter().any(|p| branch.contains(p)))
            .cloned()
            .collect();
        let mut moving = HashSet::new();
        for r in &roots {
            moving.extend(self.descendants(r));
        }
        if moving.contains(dest) {
            return Err("can't rebase onto a descendant of the branch".into());
        }
        self.check_mutable(&moving)?;
        if roots
            .iter()
            .all(|r| self.parent_ids(r) == [dest.to_string()])
        {
            return Err(format!(
                "{} is already on {}",
                self.short(rev),
                self.short(dest)
            ));
        }
        let original = self.clone();
        for id in &moving {
            self.mark(id, Preview::MovedDescendant, None);
        }
        let note = format!("rebased onto {}", self.short(dest));
        for r in &roots {
            self.node_mut(r).parents = vec![(dest.to_string(), false)];
            self.mark(r, Preview::Moved, note.clone());
        }
        self.rekey(&moving, dest);
        if ghosts {
            self.add_ghosts(&moving, &original, Preview::Ghost, "old position");
        }
        self.resort();
        Ok(())
    }

    /// `jj rebase -r rev -A target`: insert `rev` between `target` and its children.
    pub fn insert_after(&mut self, rev: &str, target: &str, ghosts: bool) -> TResult {
        self.node(rev)?;
        self.node(target)?;
        if rev == target {
            return Err("can't insert a commit after itself".into());
        }
        let mut rewritten = self.descendants(rev);
        for c in self.children(target) {
            if c != rev {
                rewritten.extend(self.descendants(&c));
            }
        }
        self.check_mutable(&rewritten)?;
        let original = self.clone();
        self.splice_out(rev);
        let target_children: Vec<String> = self
            .children(target)
            .into_iter()
            .filter(|c| c != rev)
            .collect();
        self.node_mut(rev).parents = vec![(target.to_string(), false)];
        let rev_edge = [(rev.to_string(), false)];
        for c in &target_children {
            self.replace_parent(c, target, &rev_edge);
        }
        let note = format!("inserted after {}", self.short(target));
        self.mark(rev, Preview::Moved, note);
        let set = HashSet::from([rev.to_string()]);
        self.rekey(&set, target);
        if ghosts {
            self.add_ghosts(&set, &original, Preview::Ghost, "old position");
        }
        self.resort();
        Ok(())
    }

    /// `jj rebase -r rev -B target`: insert `rev` between `target` and its parents.
    pub fn insert_before(&mut self, rev: &str, target: &str, ghosts: bool) -> TResult {
        self.node(rev)?;
        self.node(target)?;
        if rev == target {
            return Err("can't insert a commit before itself".into());
        }
        let mut rewritten = self.descendants(rev);
        rewritten.extend(self.descendants(target));
        self.check_mutable(&rewritten)?;
        if self.parent_ids(target) == [rev.to_string()] {
            return Err(format!(
                "{} is already right before {}",
                self.short(rev),
                self.short(target)
            ));
        }
        let original = self.clone();
        self.splice_out(rev);
        let target_parents = self.nodes[target].parents.clone();
        self.node_mut(rev).parents = target_parents;
        self.node_mut(target).parents = vec![(rev.to_string(), false)];
        let note = format!("inserted before {}", self.short(target));
        self.mark(rev, Preview::Moved, note);
        let set = HashSet::from([rev.to_string()]);
        self.rekey(&set, target);
        // Sit between target and its old parents.
        let k = self.nodes[target].sort_key;
        self.node_mut(rev).sort_key = k + 0.5;
        if ghosts {
            self.add_ghosts(&set, &original, Preview::Ghost, "old position");
        }
        self.resort();
        Ok(())
    }

    /// `jj squash --from from --into into`: `from` disappears, `into` gets its changes.
    pub fn squash(&mut self, from: &str, into: &str) -> TResult {
        self.node(from)?;
        self.node(into)?;
        if from == into {
            return Err("can't squash a commit into itself".into());
        }
        let mut rewritten = self.descendants(from);
        rewritten.extend(self.descendants(into));
        self.check_mutable(&rewritten)?;
        let original = self.clone();
        let was_wc = self.nodes[from].commit.is_wc;
        let from_parents = self.nodes[from].parents.clone();
        self.splice_out(from);
        let key = self.nodes[from].sort_key;
        self.nodes.remove(from);
        let set = HashSet::from([from.to_string()]);
        let note = format!("folded into {}", self.short(into));
        self.add_ghosts(&set, &original, Preview::SquashSource, &note);
        let note = format!("+ changes from {}", original.short(from));
        self.mark(into, Preview::SquashTarget, note);
        if was_wc {
            let c = self.new_wc_commit(&from_parents);
            self.add_new(NEW_WC_ID, c, from_parents, key - 0.1, "new working copy");
            self.mark(
                NEW_WC_ID,
                Preview::WcDestination,
                "new working copy".to_string(),
            );
        }
        self.resort();
        Ok(())
    }

    /// `jj abandon rev`
    pub fn abandon(&mut self, rev: &str) -> TResult {
        self.node(rev)?;
        self.check_mutable(&self.descendants(rev))?;
        let was_wc = self.nodes[rev].commit.is_wc;
        let parents = self.nodes[rev].parents.clone();
        self.splice_out(rev);
        self.node_mut(rev).commit.is_wc = false;
        self.mark(rev, Preview::Hidden, "abandoned".to_string());
        if was_wc {
            let key = self.nodes[rev].sort_key;
            let c = self.new_wc_commit(&parents);
            self.add_new(NEW_WC_ID, c, parents, key - 0.1, "new working copy");
        }
        self.resort();
        Ok(())
    }

    /// `jj abandon rev::`
    pub fn abandon_subtree(&mut self, rev: &str) -> TResult {
        self.node(rev)?;
        let set = self.descendants(rev);
        self.check_mutable(&set)?;
        let wc_inside = set.iter().any(|id| self.nodes[id].commit.is_wc);
        for id in &set {
            self.node_mut(id).commit.is_wc = false;
            self.mark(id, Preview::Hidden, None);
        }
        self.mark(
            rev,
            Preview::Hidden,
            format!("abandoned with {} descendant(s)", set.len() - 1),
        );
        if wc_inside {
            let parents = self.nodes[rev].parents.clone();
            let key = self.nodes[rev].sort_key;
            let c = self.new_wc_commit(&parents);
            self.add_new(NEW_WC_ID, c, parents, key - 0.1, "new working copy");
        }
        self.resort();
        Ok(())
    }

    /// `jj new rev`
    pub fn new_on(&mut self, rev: &str) -> TResult {
        self.node(rev)?;
        self.leave_wc();
        let parents = vec![(rev.to_string(), false)];
        let key = self.nodes[rev].sort_key - 0.4;
        let c = self.new_wc_commit(&parents);
        self.add_new(NEW_WC_ID, c, parents, key, "new change");
        self.resort();
        Ok(())
    }

    /// `jj new -A rev`
    pub fn new_after(&mut self, rev: &str) -> TResult {
        self.node(rev)?;
        let children = self.children(rev);
        for c in &children {
            self.check_mutable(&self.descendants(c))?;
        }
        self.leave_wc();
        let parents = vec![(rev.to_string(), false)];
        let key = self.nodes[rev].sort_key - 0.4;
        let c = self.new_wc_commit(&parents);
        self.add_new(NEW_WC_ID, c, parents, key, "inserted change");
        let edge = [(NEW_WC_ID.to_string(), false)];
        for c in &children {
            self.replace_parent(c, rev, &edge);
        }
        self.resort();
        Ok(())
    }

    /// `jj new -B rev`
    pub fn new_before(&mut self, rev: &str) -> TResult {
        self.node(rev)?;
        self.check_mutable(&self.descendants(rev))?;
        self.leave_wc();
        let parents = self.nodes[rev].parents.clone();
        let key = self.nodes[rev].sort_key + 0.4;
        let c = self.new_wc_commit(&parents);
        self.add_new(NEW_WC_ID, c, parents, key, "inserted change");
        self.node_mut(rev).parents = vec![(NEW_WC_ID.to_string(), false)];
        self.mark(rev, Preview::MovedDescendant, None);
        self.resort();
        Ok(())
    }

    /// `jj edit rev`
    pub fn edit(&mut self, rev: &str) -> TResult {
        let n = self.node(rev)?;
        if n.commit.is_wc {
            return Err("already editing this change".into());
        }
        if n.commit.immutable {
            return Err(format!("{} is immutable", n.commit.short_change));
        }
        self.leave_wc();
        self.node_mut(rev).commit.is_wc = true;
        self.mark(rev, Preview::WcDestination, "@ moves here".to_string());
        Ok(())
    }

    /// `jj describe rev -m message`
    pub fn describe(&mut self, rev: &str, message: &str) -> TResult {
        self.node(rev)?;
        self.check_mutable([&rev.to_string()])?;
        let n = self.node_mut(rev);
        n.commit.description = message.to_string();
        self.mark(rev, Preview::Rewritten, "new description".to_string());
        Ok(())
    }

    /// `jj duplicate rev`
    pub fn duplicate(&mut self, rev: &str) -> TResult {
        let n = self.node(rev)?.clone();
        let id = format!("dup:{rev}");
        let mut c = n.commit.clone();
        c.is_wc = false;
        c.immutable = false;
        c.trunk = false;
        c.bookmarks.clear();
        c.remote_bookmarks.clear();
        c.short_change = "(new)".into();
        let note = format!("duplicate of {}", n.commit.short_change);
        self.add_new(&id, c, n.parents.clone(), n.sort_key - 0.4, &note);
        self.resort();
        Ok(())
    }

    /// `jj revert -r rev -o onto`
    pub fn revert(&mut self, rev: &str, onto: &str) -> TResult {
        let n = self.node(rev)?.clone();
        self.node(onto)?;
        let id = format!("revert:{rev}");
        let c = Commit::synthetic(&id, &format!("Revert \"{}\"", n.commit.title()));
        let key = self.nodes[onto].sort_key - 0.4;
        let note = format!("reverts {}", n.commit.short_change);
        self.add_new(&id, c, vec![(onto.to_string(), false)], key, &note);
        self.resort();
        Ok(())
    }

    /// `jj bookmark set name -r target`
    pub fn bookmark_set(&mut self, name: &str, target: &str) -> TResult {
        self.node(target)?;
        let old = self
            .order
            .iter()
            .find(|id| self.nodes[*id].commit.bookmarks.iter().any(|b| b == name))
            .cloned();
        if old.as_deref() == Some(target) {
            return Err(format!("{name} already points here"));
        }
        if let Some(old) = old {
            self.node_mut(&old).commit.bookmarks.retain(|b| b != name);
            self.mark(&old, Preview::BookmarkOld, format!("{name} moves away"));
        }
        self.node_mut(target)
            .commit
            .bookmarks
            .push(name.to_string());
        self.mark(target, Preview::BookmarkNew, format!("← {name}"));
        Ok(())
    }

    /// Rebase what's built on shipped commits onto trunk, then abandon them.
    pub fn cleanup_shipped(&mut self, shipped: &HashSet<String>, trunk: &str) -> TResult {
        let mut roots = vec![];
        for id in shipped {
            for c in self.children(id) {
                if !shipped.contains(&c) && !roots.contains(&c) {
                    roots.push(c);
                }
            }
        }
        for r in &roots {
            if self.parent_ids(r) != [trunk.to_string()] {
                self.check_mutable(&self.descendants(r))?;
                self.node_mut(r).parents = vec![(trunk.to_string(), false)];
                let note = format!("rebased onto {}", self.short(trunk));
                self.mark(r, Preview::Moved, note);
            }
        }
        for id in shipped {
            if self.nodes[id].commit.is_wc {
                let c = self.new_wc_commit(&[(trunk.to_string(), false)]);
                let key = self.nodes[trunk].sort_key - 0.4;
                self.add_new(
                    NEW_WC_ID,
                    c,
                    vec![(trunk.to_string(), false)],
                    key,
                    "new working copy",
                );
            }
            self.node_mut(id).commit.is_wc = false;
            self.mark(id, Preview::Hidden, "shipped — abandoned".to_string());
        }
        self.resort();
        Ok(())
    }

    pub fn annotate(&mut self, id: &str, preview: Preview, note: String) {
        self.mark(id, preview, note);
    }

    /// Recomputes `order`: children always before parents, chains kept
    /// contiguous, heads ordered by `sort_key`.
    pub fn resort(&mut self) {
        // Drop edges to nodes that no longer exist.
        let ids: HashSet<String> = self.nodes.keys().cloned().collect();
        for n in self.nodes.values_mut() {
            n.parents.retain(|(p, _)| ids.contains(p));
        }
        let mut pending: HashMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
        for n in self.nodes.values() {
            for (p, _) in &n.parents {
                *pending.get_mut(p).expect("parent exists") += 1;
            }
        }
        let mut out = Vec::with_capacity(ids.len());
        let mut emitted: HashSet<String> = HashSet::new();
        let mut prefer: Vec<String> = vec![];
        let ready_min = |emitted: &HashSet<String>,
                         pending: &HashMap<String, usize>,
                         within: Option<&HashSet<String>>| {
            self.nodes
                .values()
                .filter(|n| pending[&n.id] == 0 && !emitted.contains(&n.id))
                .filter(|n| within.is_none_or(|w| w.contains(&n.id)))
                .min_by(|a, b| {
                    a.sort_key
                        .total_cmp(&b.sort_key)
                        .then_with(|| a.id.cmp(&b.id))
                })
                .map(|n| n.id.clone())
        };
        while out.len() < ids.len() {
            let mut next = None;
            // Continue down the current chain; if its next commit still has
            // unemitted children, emit those branches first (Sapling-style
            // graph sort) so they attach right above it.
            while let Some(p) = prefer.last().cloned() {
                if emitted.contains(&p) {
                    prefer.pop();
                    continue;
                }
                if pending[&p] == 0 {
                    prefer.pop();
                    next = Some(p);
                    break;
                }
                let below = self.descendants(&p);
                match ready_min(&emitted, &pending, Some(&below)) {
                    Some(r) => {
                        next = Some(r);
                        break;
                    }
                    None => {
                        prefer.pop();
                    }
                }
            }
            let next = next.or_else(|| ready_min(&emitted, &pending, None));
            // A cycle can't come from jj; bail out deterministically if a
            // transform ever produced one.
            let next = match next {
                Some(n) => n,
                None => {
                    let mut rest: Vec<&Node> = self
                        .nodes
                        .values()
                        .filter(|n| !emitted.contains(&n.id))
                        .collect();
                    rest.sort_by(|a, b| a.sort_key.total_cmp(&b.sort_key));
                    out.extend(rest.into_iter().map(|n| n.id.clone()));
                    break;
                }
            };
            emitted.insert(next.clone());
            let parents = self.parent_ids(&next);
            // A parent some earlier chain is already waiting on goes on top,
            // so that chain closes without crossing other lanes; otherwise
            // the first parent continues the current chain.
            let (waiting, fresh): (Vec<&String>, Vec<&String>) =
                parents.iter().partition(|p| prefer.contains(p));
            for p in fresh.iter().rev().chain(waiting.iter().rev()) {
                *pending.get_mut(*p).expect("parent exists") -= 1;
                prefer.push((*p).clone());
            }
            out.push(next);
        }
        self.order = out;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::jj::{GraphEdges, RepoGraph};

    /// Builds a graph from `(id, parents)` in display order; ids starting
    /// with `T` are immutable, `@` marks the working copy.
    pub fn graph(spec: &[(&str, &[&str])]) -> Dag {
        let mut commits = vec![];
        let mut edges = HashMap::new();
        for (id, parents) in spec {
            let raw = id.trim_start_matches('@');
            let mut c = Commit::synthetic(raw, &format!("commit {raw}"));
            c.short_change = raw.to_string();
            c.empty = false;
            c.is_wc = id.starts_with('@');
            c.immutable = raw.starts_with('T');
            c.trunk = raw == "T";
            c.parents = parents.iter().map(|p| p.to_string()).collect();
            edges.insert(
                raw.to_string(),
                GraphEdges {
                    parents: parents.iter().map(|p| (p.to_string(), false)).collect(),
                    truncated: false,
                },
            );
            commits.push(c);
        }
        Dag::from_repo(&RepoGraph { commits, edges })
    }

    fn parents(d: &Dag, id: &str) -> Vec<String> {
        d.parent_ids(id)
    }

    fn assert_topo(d: &Dag) {
        let pos: HashMap<&str, usize> = d
            .order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        for n in d.nodes.values() {
            for (p, _) in &n.parents {
                assert!(
                    pos[n.id.as_str()] < pos[p.as_str()],
                    "{} must come before parent {p}: {:?}",
                    n.id,
                    d.order
                );
            }
        }
        assert_eq!(d.order.len(), d.nodes.len());
    }

    // T <- a <- b <- @c ; T <- x
    fn sample() -> Dag {
        graph(&[
            ("@c", &["b"]),
            ("b", &["a"]),
            ("x", &["T"]),
            ("a", &["T"]),
            ("T", &[]),
        ])
    }

    #[test]
    fn rebase_source_moves_subtree() {
        let mut d = sample();
        d.rebase_source("b", "x", true).unwrap();
        assert_eq!(parents(&d, "b"), vec!["x"]);
        assert_eq!(parents(&d, "c"), vec!["b"]);
        assert_eq!(d.nodes["b"].preview, Some(Preview::Moved));
        assert_eq!(d.nodes["c"].preview, Some(Preview::MovedDescendant));
        assert_eq!(parents(&d, "ghost:b"), vec!["a"]);
        assert_eq!(parents(&d, "ghost:c"), vec!["ghost:b"]);
        assert_topo(&d);
        // Moved subtree sits directly above its new parent.
        let pos = |id: &str| d.order.iter().position(|x| x == id).unwrap();
        assert_eq!(pos("b") + 1, pos("x"));
    }

    #[test]
    fn rebase_rejects_descendant_and_immutable() {
        let mut d = sample();
        assert!(d.rebase_source("a", "c", false).is_err());
        assert!(d
            .clone()
            .rebase_revision("T", "x", false)
            .unwrap_err()
            .contains("immutable"));
        assert!(d
            .rebase_source("b", "a", false)
            .unwrap_err()
            .contains("already"));
    }

    #[test]
    fn rebase_revision_leaves_children() {
        let mut d = sample();
        d.rebase_revision("b", "x", false).unwrap();
        assert_eq!(parents(&d, "b"), vec!["x"]);
        assert_eq!(parents(&d, "c"), vec!["a"]);
        assert_topo(&d);
    }

    #[test]
    fn rebase_branch_moves_roots() {
        let mut d = sample();
        d.rebase_branch("c", "x", false).unwrap();
        assert_eq!(parents(&d, "a"), vec!["x"]);
        assert_eq!(parents(&d, "c"), vec!["b"]);
        assert_topo(&d);
    }

    #[test]
    fn insert_after_and_before() {
        let mut d = sample();
        d.insert_after("x", "a", false).unwrap();
        assert_eq!(parents(&d, "x"), vec!["a"]);
        assert_eq!(parents(&d, "b"), vec!["x"]);
        assert_topo(&d);

        let mut d = sample();
        d.insert_before("x", "b", false).unwrap();
        assert_eq!(parents(&d, "x"), vec!["a"]);
        assert_eq!(parents(&d, "b"), vec!["x"]);
        assert_topo(&d);
    }

    #[test]
    fn squash_wc_into_parent_creates_new_wc() {
        let mut d = sample();
        d.squash("c", "b").unwrap();
        assert!(!d.nodes.contains_key("c"));
        assert_eq!(d.nodes["ghost:c"].preview, Some(Preview::SquashSource));
        assert_eq!(d.nodes["b"].preview, Some(Preview::SquashTarget));
        assert_eq!(parents(&d, NEW_WC_ID), vec!["b"]);
        assert!(d.nodes[NEW_WC_ID].commit.is_wc);
        assert_topo(&d);
    }

    #[test]
    fn squash_middle_reparents_children() {
        let mut d = sample();
        d.squash("b", "a").unwrap();
        assert_eq!(parents(&d, "c"), vec!["a"]);
        assert!(d.squash("a", "T").unwrap_err().contains("immutable"));
    }

    #[test]
    fn abandon_reparents() {
        let mut d = sample();
        d.abandon("b").unwrap();
        assert_eq!(parents(&d, "c"), vec!["a"]);
        assert_eq!(d.nodes["b"].preview, Some(Preview::Hidden));
        assert_topo(&d);
    }

    #[test]
    fn new_and_edit_move_wc() {
        let mut d = sample();
        d.new_on("x").unwrap();
        assert!(d.nodes[NEW_WC_ID].commit.is_wc);
        assert!(!d.nodes["c"].commit.is_wc);
        assert_eq!(d.nodes["c"].preview, Some(Preview::WcPrevious));

        let mut d = sample();
        d.edit("a").unwrap();
        assert!(d.nodes["a"].commit.is_wc);
        assert!(d.edit("T").is_err());
    }

    #[test]
    fn new_before_inserts() {
        let mut d = sample();
        d.new_before("b").unwrap();
        assert_eq!(parents(&d, "b"), vec![NEW_WC_ID]);
        assert_eq!(parents(&d, NEW_WC_ID), vec!["a"]);
        assert_topo(&d);
    }

    #[test]
    fn cleanup_rebases_survivors() {
        let mut d = sample();
        let shipped = HashSet::from(["a".to_string()]);
        d.cleanup_shipped(&shipped, "T").unwrap();
        assert_eq!(parents(&d, "b"), vec!["T"]);
        assert_eq!(d.nodes["a"].preview, Some(Preview::Hidden));
    }

    #[test]
    fn bookmark_moves() {
        let mut d = sample();
        d.nodes
            .get_mut("a")
            .unwrap()
            .commit
            .bookmarks
            .push("feat".into());
        d.bookmark_set("feat", "c").unwrap();
        assert!(d.nodes["c"].commit.bookmarks.contains(&"feat".to_string()));
        assert!(d.nodes["a"].commit.bookmarks.is_empty());
    }
}
