//! Local jj operations: what command each runs and how it is previewed.

use std::collections::HashSet;

use crate::dag::Dag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// `-s`: the commit and its descendants.
    Source,
    /// `-r`: only the commit.
    Revision,
    /// `-b`: the whole branch relative to the destination.
    Branch,
}

impl Scope {
    pub fn next(self) -> Scope {
        match self {
            Scope::Source => Scope::Revision,
            Scope::Revision => Scope::Branch,
            Scope::Branch => Scope::Source,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Scope::Source => "-s commit + descendants",
            Scope::Revision => "-r only this commit",
            Scope::Branch => "-b whole branch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Onto,
    After,
    Before,
}

impl Placement {
    pub fn next(self) -> Placement {
        match self {
            Placement::Onto => Placement::After,
            Placement::After => Placement::Before,
            Placement::Before => Placement::Onto,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Placement::Onto => "onto (-o)",
            Placement::After => "insert after (-A)",
            Placement::Before => "insert before (-B)",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Rebase {
        scope: Scope,
        placement: Placement,
        rev: String,
        target: String,
    },
    Squash {
        from: String,
        into: String,
    },
    Abandon {
        rev: String,
    },
    AbandonSubtree {
        rev: String,
    },
    New {
        rev: String,
    },
    NewAfter {
        rev: String,
    },
    NewBefore {
        rev: String,
    },
    Edit {
        rev: String,
    },
    Describe {
        rev: String,
        message: String,
    },
    Duplicate {
        rev: String,
    },
    Revert {
        rev: String,
        onto: String,
    },
    BookmarkSet {
        name: String,
        target: String,
    },
    Cleanup {
        shipped: Vec<String>,
        trunk: String,
    },
    Undo,
    Redo,
    /// Push every bookmark on `revset`.
    Push {
        revset: String,
        remote: String,
    },
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

/// Combines descriptions the way VisualJJ's squash does, so jj never needs
/// to open an editor.
pub fn squash_message(from: &str, into: &str) -> Option<String> {
    let (from, into) = (from.trim(), into.trim());
    match (from.is_empty(), into.is_empty()) {
        (true, _) => None,
        (false, true) => Some(from.to_string()),
        (false, false) => Some(format!("{into}\n\n{from}")),
    }
}

impl Op {
    /// jj invocations (without the leading `jj`) that perform this operation.
    pub fn commands(&self, dag: &Dag) -> Vec<Vec<String>> {
        match self {
            Op::Rebase {
                scope,
                placement,
                rev,
                target,
            } => {
                // -A/-B only make sense for single revisions in jj's UI; for
                // -s/-b they still work (they move the whole set).
                let scope_flag = match scope {
                    Scope::Source => "-s",
                    Scope::Revision => "-r",
                    Scope::Branch => "-b",
                };
                let place_flag = match placement {
                    Placement::Onto => "-o",
                    Placement::After => "-A",
                    Placement::Before => "-B",
                };
                vec![s(&["rebase", scope_flag, rev, place_flag, target])]
            }
            Op::Squash { from, into } => {
                let desc = |id: &str| {
                    dag.get(id)
                        .map(|n| n.commit.description.clone())
                        .unwrap_or_default()
                };
                let mut cmd = s(&["squash", "--from", from, "--into", into]);
                match squash_message(&desc(from), &desc(into)) {
                    None => cmd.push("--use-destination-message".into()),
                    Some(m) => {
                        cmd.push("-m".into());
                        cmd.push(m);
                    }
                }
                vec![cmd]
            }
            Op::Abandon { rev } => vec![s(&["abandon", rev])],
            Op::AbandonSubtree { rev } => vec![s(&["abandon", &format!("{rev}::")])],
            Op::New { rev } => vec![s(&["new", rev])],
            Op::NewAfter { rev } => vec![s(&["new", "-A", rev])],
            Op::NewBefore { rev } => vec![s(&["new", "-B", rev])],
            Op::Edit { rev } => vec![s(&["edit", rev])],
            Op::Describe { rev, message } => vec![s(&["describe", rev, "-m", message])],
            Op::Duplicate { rev } => vec![s(&["duplicate", rev])],
            Op::Revert { rev, onto } => vec![s(&["revert", "-r", rev, "-o", onto])],
            Op::BookmarkSet { name, target } => {
                vec![s(&[
                    "bookmark",
                    "set",
                    name,
                    "-r",
                    target,
                    "--allow-backwards",
                ])]
            }
            Op::Cleanup { shipped, trunk } => {
                let set = shipped.join(" | ");
                let mut cmds = vec![];
                let survivors = format!("children({set}) ~ ({set})");
                let survivors_exist = shipped
                    .iter()
                    .any(|id| dag.children(id).iter().any(|c| !shipped.contains(c)));
                if survivors_exist {
                    cmds.push(s(&["rebase", "-s", &survivors, "-o", trunk]));
                }
                cmds.push(s(&["abandon", &set]));
                cmds
            }
            Op::Undo => vec![s(&["undo"])],
            Op::Redo => vec![s(&["redo"])],
            Op::Push { revset, remote } => {
                vec![s(&["git", "push", "--remote", remote, "-r", revset])]
            }
        }
    }

    /// The graph after this operation, or why it can't run.
    pub fn preview(&self, dag: &Dag, ghosts: bool) -> Result<Dag, String> {
        let mut d = dag.clone();
        match self {
            Op::Rebase {
                scope,
                placement,
                rev,
                target,
            } => match (scope, placement) {
                (Scope::Source, Placement::Onto) => d.rebase_source(rev, target, ghosts)?,
                (Scope::Revision, Placement::Onto) => d.rebase_revision(rev, target, ghosts)?,
                (Scope::Branch, Placement::Onto) => d.rebase_branch(rev, target, ghosts)?,
                (Scope::Revision, Placement::After) => d.insert_after(rev, target, ghosts)?,
                (Scope::Revision, Placement::Before) => d.insert_before(rev, target, ghosts)?,
                (_, _) => {
                    return Err("insert after/before needs -r (only this commit); press m".into())
                }
            },
            Op::Squash { from, into } => d.squash(from, into)?,
            Op::Abandon { rev } => d.abandon(rev)?,
            Op::AbandonSubtree { rev } => d.abandon_subtree(rev)?,
            Op::New { rev } => d.new_on(rev)?,
            Op::NewAfter { rev } => d.new_after(rev)?,
            Op::NewBefore { rev } => d.new_before(rev)?,
            Op::Edit { rev } => d.edit(rev)?,
            Op::Describe { rev, message } => d.describe(rev, message)?,
            Op::Duplicate { rev } => d.duplicate(rev)?,
            Op::Revert { rev, onto } => d.revert(rev, onto)?,
            Op::BookmarkSet { name, target } => d.bookmark_set(name, target)?,
            Op::Cleanup { shipped, trunk } => {
                let set: HashSet<String> = shipped.iter().cloned().collect();
                d.cleanup_shipped(&set, trunk)?
            }
            Op::Undo | Op::Redo | Op::Push { .. } => {}
        }
        Ok(d)
    }

    pub fn title(&self, dag: &Dag) -> String {
        let sh = |id: &str| dag.short(id);
        match self {
            Op::Rebase {
                scope,
                placement,
                rev,
                target,
            } => {
                let what = match scope {
                    Scope::Source => {
                        let n = dag.descendants(rev).len() - 1;
                        if n == 0 {
                            sh(rev)
                        } else {
                            format!(
                                "{} (+{n} descendant{})",
                                sh(rev),
                                if n == 1 { "" } else { "s" }
                            )
                        }
                    }
                    Scope::Revision => format!("only {}", sh(rev)),
                    Scope::Branch => format!("branch of {}", sh(rev)),
                };
                let place = match placement {
                    Placement::Onto => "onto",
                    Placement::After => "after",
                    Placement::Before => "before",
                };
                format!("Rebase {what} {place} {}", sh(target))
            }
            Op::Squash { from, into } => format!("Squash {} into {}", sh(from), sh(into)),
            Op::Abandon { rev } => format!("Abandon {}", sh(rev)),
            Op::AbandonSubtree { rev } => format!("Abandon {} and descendants", sh(rev)),
            Op::New { rev } => format!("New change on {}", sh(rev)),
            Op::NewAfter { rev } => format!("Insert new change after {}", sh(rev)),
            Op::NewBefore { rev } => format!("Insert new change before {}", sh(rev)),
            Op::Edit { rev } => format!("Edit {}", sh(rev)),
            Op::Describe { rev, .. } => format!("Describe {}", sh(rev)),
            Op::Duplicate { rev } => format!("Duplicate {}", sh(rev)),
            Op::Revert { rev, onto } => format!("Revert {} onto {}", sh(rev), sh(onto)),
            Op::BookmarkSet { name, target } => format!("Set bookmark {name} to {}", sh(target)),
            Op::Cleanup { shipped, .. } => format!("Clean up {} shipped commit(s)", shipped.len()),
            Op::Undo => "Undo last operation".into(),
            Op::Redo => "Redo".into(),
            Op::Push { remote, .. } => format!("Push this stack's bookmarks to {remote}"),
        }
    }
}

/// Operations that wait for the user to pick a target commit.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    Rebase { scope: Scope, placement: Placement },
    SquashInto,
    RevertOnto,
    BookmarkMove { name: String },
}

impl Pick {
    pub fn build(&self, rev: &str, target: &str) -> Op {
        match self {
            Pick::Rebase { scope, placement } => Op::Rebase {
                scope: *scope,
                placement: *placement,
                rev: rev.into(),
                target: target.into(),
            },
            Pick::SquashInto => Op::Squash {
                from: rev.into(),
                into: target.into(),
            },
            Pick::RevertOnto => Op::Revert {
                rev: rev.into(),
                onto: target.into(),
            },
            Pick::BookmarkMove { name } => Op::BookmarkSet {
                name: name.clone(),
                target: target.into(),
            },
        }
    }

    pub fn label(&self) -> String {
        match self {
            Pick::Rebase { scope, placement } => {
                format!("Rebase [{}] [{}]", scope.label(), placement.label())
            }
            Pick::SquashInto => "Squash into…".into(),
            Pick::RevertOnto => "Revert onto…".into(),
            Pick::BookmarkMove { name } => format!("Move bookmark {name} to…"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::tests::graph;

    #[test]
    fn rebase_commands() {
        let d = graph(&[
            ("@c", &["b"]),
            ("b", &["a"]),
            ("x", &["T"]),
            ("a", &["T"]),
            ("T", &[]),
        ]);
        let op = Op::Rebase {
            scope: Scope::Source,
            placement: Placement::Onto,
            rev: "b".into(),
            target: "x".into(),
        };
        assert_eq!(op.commands(&d), vec![s(&["rebase", "-s", "b", "-o", "x"])]);
        assert_eq!(op.title(&d), "Rebase b (+1 descendant) onto x");
        assert!(op.preview(&d, true).is_ok());
        let bad = Op::Rebase {
            scope: Scope::Source,
            placement: Placement::After,
            rev: "b".into(),
            target: "x".into(),
        };
        assert!(bad.preview(&d, true).is_err());
    }

    #[test]
    fn squash_message_combines() {
        assert_eq!(squash_message("", "into"), None);
        assert_eq!(squash_message("from", ""), Some("from".into()));
        assert_eq!(squash_message("from", "into"), Some("into\n\nfrom".into()));
        let d = graph(&[("@c", &["b"]), ("b", &["a"]), ("a", &["T"]), ("T", &[])]);
        let cmds = Op::Squash {
            from: "c".into(),
            into: "b".into(),
        }
        .commands(&d);
        assert_eq!(
            cmds[0][..5],
            s(&["squash", "--from", "c", "--into", "b"])[..]
        );
        assert_eq!(cmds[0][5], "-m");
        assert_eq!(cmds[0][6], "commit b\n\ncommit c");
    }

    #[test]
    fn cleanup_commands() {
        let d = graph(&[("@c", &["b"]), ("b", &["a"]), ("a", &["T"]), ("T", &[])]);
        let op = Op::Cleanup {
            shipped: vec!["a".into()],
            trunk: "T".into(),
        };
        let cmds = op.commands(&d);
        assert_eq!(
            cmds[0],
            s(&["rebase", "-s", "children(a) ~ (a)", "-o", "T"])
        );
        assert_eq!(cmds[1], s(&["abandon", "a"]));
    }
}
