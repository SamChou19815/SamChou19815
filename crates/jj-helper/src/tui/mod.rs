//! Interactive TUI: a commit graph you act on by selecting a commit, picking
//! a command, and (for commands with a destination) picking the target while
//! a live "after" preview shows the resulting graph next to the current one.

mod render;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use crate::dag::{Dag, Preview};
use crate::github::{open_in_browser, GitHub, PrInfo, PrState};
use crate::jj::{shell_join, Jj, RepoGraph};
use crate::ops::{Op, Pick, Placement, Scope};
use crate::stack::{self, Plan, SubmitOptions};
use render::{diff_lines, pr_badge, pr_for, relative_time, Mark, Rendered};

#[derive(Clone, Debug)]
pub struct Config {
    pub revset: String,
    pub remote: String,
    pub submit: SubmitOptions,
    pub merge_method: String,
    pub auto_fetch: Option<Duration>,
    pub auto_rebase: bool,
    pub refresh: Option<Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    Edit,
    New,
    NewBefore,
    NewAfter,
    Describe,
    Rebase,
    RebaseTrunk,
    Squash,
    SquashInto,
    Split,
    Abandon,
    AbandonSubtree,
    Duplicate,
    Revert,
    Bookmark,
    Diff,
    Interdiff,
    Submit,
    Ship,
    OpenPr,
    Cleanup,
    Fetch,
    Push,
    Undo,
    Redo,
    Refresh,
    ToggleAutoRebase,
    Help,
    Quit,
}

const COMMANDS: &[(&str, char, Cmd, &str)] = &[
    ("Change", 'e', Cmd::Edit, "Edit this change (@ moves here)"),
    ("Change", 'n', Cmd::New, "New change on top"),
    ("Change", 'N', Cmd::NewBefore, "Insert new change before"),
    ("Change", 'I', Cmd::NewAfter, "Insert new change after"),
    ("Change", 'd', Cmd::Describe, "Describe…"),
    ("Change", 'b', Cmd::Bookmark, "Set / move bookmark…"),
    ("Change", 'D', Cmd::Diff, "Show diff"),
    ("Shape", 'r', Cmd::Rebase, "Rebase… (pick destination)"),
    ("Shape", 't', Cmd::RebaseTrunk, "Rebase stack onto trunk"),
    ("Shape", 's', Cmd::Squash, "Squash into parent"),
    (
        "Shape",
        'S',
        Cmd::SquashInto,
        "Squash into… (pick destination)",
    ),
    ("Shape", 'x', Cmd::Split, "Split (interactive)"),
    ("Shape", 'y', Cmd::Duplicate, "Duplicate"),
    ("Shape", 'v', Cmd::Revert, "Revert onto… (pick destination)"),
    ("Shape", 'a', Cmd::Abandon, "Abandon"),
    (
        "Shape",
        'A',
        Cmd::AbandonSubtree,
        "Abandon with descendants",
    ),
    ("GitHub", 'P', Cmd::Submit, "Create / update stacked PRs"),
    (
        "GitHub",
        'M',
        Cmd::Ship,
        "Ship (merge) PR + everything below",
    ),
    ("GitHub", 'o', Cmd::OpenPr, "Open PR in browser"),
    (
        "GitHub",
        'i',
        Cmd::Interdiff,
        "Interdiff since push / review",
    ),
    ("GitHub", 'c', Cmd::Cleanup, "Clean up shipped commits"),
    ("Repo", 'f', Cmd::Fetch, "Fetch (+ safe auto-rebase if on)"),
    ("Repo", 'p', Cmd::Push, "Push this stack's bookmarks"),
    ("Repo", 'u', Cmd::Undo, "Undo last jj operation"),
    ("Repo", 'U', Cmd::Redo, "Redo"),
    ("Repo", 'R', Cmd::Refresh, "Refresh"),
    (
        "Repo",
        'T',
        Cmd::ToggleAutoRebase,
        "Toggle auto-rebase after fetch",
    ),
    ("Repo", '?', Cmd::Help, "Help"),
    ("Repo", 'q', Cmd::Quit, "Quit"),
];

enum Action {
    Op(Op),
    Submit(Box<Plan>),
    Ship { pr: Box<PrInfo>, commit_id: String },
}

enum InputNext {
    Describe { rev: String, body: String },
    Bookmark { rev: String },
}

#[derive(Clone)]
enum Choice {
    Interdiff {
        from: String,
        to: String,
        title: String,
    },
}

enum Mode {
    Normal,
    Menu {
        state: ListState,
    },
    Pick {
        pick: Pick,
        rev: String,
        target: usize,
    },
    Confirm {
        action: Action,
    },
    Input {
        prompt: String,
        buffer: String,
        next: InputNext,
    },
    Choose {
        title: String,
        items: Vec<(String, Choice)>,
        state: ListState,
    },
    Text {
        title: String,
        lines: Vec<Line<'static>>,
        scroll: usize,
    },
    Help {
        scroll: usize,
    },
}

enum Outcome {
    Message(String),
    Plan(Box<Plan>),
    Prs(HashMap<String, PrInfo>),
}

enum JobMsg {
    Log(String),
    Done(Result<Outcome, String>),
}

struct Job {
    title: String,
    rx: Receiver<JobMsg>,
    started: Instant,
    log: Vec<String>,
    /// Background jobs (PR loading, auto-fetch) don't block commands.
    background: bool,
}

/// Preview graph, node to keep in view, commands/plan lines, title.
type BuiltPreview = (Result<Dag, String>, Option<String>, Vec<String>, String);

/// Cached preview for the current Pick/Confirm state.
struct PreviewState {
    key: String,
    result: Result<Rendered, String>,
    focus: Option<String>,
    commands: Vec<String>,
    title: String,
}

pub struct App {
    jj: Jj,
    cfg: Config,
    repo: RepoGraph,
    dag: Dag,
    rendered: Rendered,
    cursor: usize,
    scroll: usize,
    preview_scroll: usize,
    mode: Mode,
    ghosts: bool,
    prs: HashMap<String, PrInfo>,
    gh: Option<Arc<GitHub>>,
    gh_error: Option<String>,
    trunk_branch: String,
    message: (String, Color),
    job: Option<Job>,
    pr_job: Option<Job>,
    last_fetch: Instant,
    last_refresh: Instant,
    last_pr_load: Option<Instant>,
    preview: Option<PreviewState>,
    files_cache: HashMap<String, String>,
    interactive: Option<Vec<String>>,
    root: String,
    quit: bool,
}

fn spawn_job(
    title: impl Into<String>,
    background: bool,
    f: impl FnOnce(&mut dyn FnMut(String)) -> Result<Outcome> + Send + 'static,
) -> Job {
    let (tx, rx): (Sender<JobMsg>, Receiver<JobMsg>) = channel();
    std::thread::spawn(move || {
        let log_tx = tx.clone();
        let mut log = move |s: String| {
            let _ = log_tx.send(JobMsg::Log(s));
        };
        let result = f(&mut log).map_err(|e| format!("{e:#}"));
        let _ = tx.send(JobMsg::Done(result));
    });
    Job {
        title: title.into(),
        rx,
        started: Instant::now(),
        log: vec![],
        background,
    }
}

pub fn run(jj: Jj, cfg: Config) -> Result<()> {
    let repo = jj.load_graph(&cfg.revset)?;
    let trunk_branch = jj.trunk_branch();
    let (gh, gh_error) = match GitHub::for_remote(&jj, &cfg.remote) {
        Ok(g) => (Some(Arc::new(g)), None),
        Err(e) => (None, Some(format!("{e:#}"))),
    };
    let dag = Dag::from_repo(&repo);
    let rendered = render::render(&dag, &HashMap::new(), &HashMap::new());
    let mut app = App {
        jj,
        cfg,
        repo,
        dag,
        rendered,
        cursor: 0,
        scroll: 0,
        preview_scroll: 0,
        mode: Mode::Normal,
        ghosts: true,
        prs: HashMap::new(),
        gh,
        gh_error,
        trunk_branch,
        message: (String::new(), Color::Reset),
        job: None,
        pr_job: None,
        last_fetch: Instant::now(),
        last_refresh: Instant::now(),
        last_pr_load: None,
        preview: None,
        files_cache: HashMap::new(),
        interactive: None,
        root: String::new(),
        quit: false,
    };
    app.root = app
        .jj
        .workspace_root()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    app.cursor = app
        .dag
        .wc()
        .and_then(|wc| app.dag.order.iter().position(|id| *id == wc))
        .unwrap_or(0);
    app.rerender();
    app.load_prs();
    if app.gh.is_none() {
        let err = app.gh_error.clone().unwrap_or_default();
        app.set_message(format!("GitHub disabled: {err}"), Color::Yellow);
    }

    let mut terminal = ratatui::init();
    let result = app.event_loop(&mut terminal);
    ratatui::restore();
    result
}

impl App {
    fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;
            if event::poll(Duration::from_millis(120))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        self.on_key(k);
                    }
                }
            }
            self.tick();
            if let Some(cmd) = self.interactive.take() {
                ratatui::restore();
                let result = self.jj.run_interactive(&cmd);
                *terminal = ratatui::init();
                terminal.clear()?;
                match result {
                    Ok(()) => self.set_message(format!("✓ jj {}", shell_join(&cmd)), Color::Green),
                    Err(e) => self.set_message(format!("✗ {e:#}"), Color::Red),
                }
                self.refresh();
            }
            if self.quit {
                return Ok(());
            }
        }
    }

    // ---- state helpers -----------------------------------------------------

    fn set_message(&mut self, msg: impl Into<String>, color: Color) {
        self.message = (msg.into(), color);
    }

    fn selected(&self) -> Option<String> {
        self.dag.order.get(self.cursor).cloned()
    }

    fn short(&self, id: &str) -> String {
        self.dag.short(id)
    }

    fn busy(&mut self) -> bool {
        if self.job.as_ref().is_some_and(|j| !j.background) {
            self.set_message(
                "busy — wait for the running command to finish",
                Color::Yellow,
            );
            return true;
        }
        false
    }

    fn rerender(&mut self) {
        let mut marks = HashMap::new();
        if let Some(id) = self.selected() {
            marks.insert(id, Mark::Cursor);
        }
        if let Mode::Pick { rev, target, .. } = &self.mode {
            marks.insert(rev.clone(), Mark::Source);
            if let Some(t) = self.dag.order.get(*target) {
                marks.insert(t.clone(), Mark::Target);
            }
        }
        if let Mode::Confirm {
            action: Action::Op(_),
        } = &self.mode
        {
            if let Some(id) = self.selected() {
                marks.insert(id, Mark::Source);
            }
        }
        self.rendered = render::render(&self.dag, &self.prs, &marks);
        self.update_preview();
    }

    fn refresh(&mut self) {
        let keep = self
            .selected()
            .and_then(|id| self.dag.get(&id).map(|n| n.commit.change_id.clone()));
        match self.jj.load_graph(&self.cfg.revset) {
            Ok(repo) => {
                self.repo = repo;
                self.dag = Dag::from_repo(&self.repo);
                if let Some(change) = keep {
                    if let Some(i) = self
                        .dag
                        .order
                        .iter()
                        .position(|id| self.dag.nodes[id].commit.change_id == change)
                    {
                        self.cursor = i;
                    }
                }
                self.cursor = self.cursor.min(self.dag.order.len().saturating_sub(1));
                self.files_cache.clear();
            }
            Err(e) => self.set_message(format!("✗ refresh failed: {e:#}"), Color::Red),
        }
        self.last_refresh = Instant::now();
        self.rerender();
    }

    /// Loads PR status for every bookmark in view (in the background).
    fn load_prs(&mut self) {
        let Some(gh) = self.gh.clone() else { return };
        if self.pr_job.is_some() {
            return;
        }
        let mut branches: Vec<String> = self
            .dag
            .nodes
            .values()
            .filter(|n| !n.commit.trunk)
            .flat_map(|n| n.commit.bookmarks.iter().cloned())
            .filter(|b| *b != self.trunk_branch)
            .collect();
        branches.sort();
        branches.dedup();
        self.last_pr_load = Some(Instant::now());
        if branches.is_empty() {
            return;
        }
        self.pr_job = Some(spawn_job("loading PRs", true, move |_| {
            Ok(Outcome::Prs(gh.prs_for_branches(&branches)?))
        }));
    }

    fn tick(&mut self) {
        // Main job.
        let mut finished = None;
        if let Some(job) = &mut self.job {
            while let Ok(msg) = job.rx.try_recv() {
                match msg {
                    JobMsg::Log(s) => job.log.push(s),
                    JobMsg::Done(r) => finished = Some(r),
                }
            }
        }
        if let Some(result) = finished {
            let job = self.job.take().expect("job present");
            self.finish_job(job, result);
        }
        // PR loading.
        let mut prs_done = None;
        if let Some(job) = &mut self.pr_job {
            while let Ok(msg) = job.rx.try_recv() {
                if let JobMsg::Done(r) = msg {
                    prs_done = Some(r);
                }
            }
        }
        if let Some(r) = prs_done {
            self.pr_job = None;
            match r {
                Ok(Outcome::Prs(prs)) => {
                    self.prs = prs;
                    self.rerender();
                    let shipped = self.shipped_commits();
                    if !shipped.is_empty()
                        && matches!(self.mode, Mode::Normal)
                        && self.message.0.is_empty()
                    {
                        self.set_message(
                            format!(
                                "{} commit(s) have merged PRs — press c to clean up",
                                shipped.len()
                            ),
                            Color::Magenta,
                        );
                    }
                }
                Ok(_) => {}
                Err(e) => self.set_message(format!("✗ PR status: {e}"), Color::Red),
            }
        }
        let idle = self.job.is_none() && matches!(self.mode, Mode::Normal);
        // Pro: automatic fetching (+ safe rebase).
        if let Some(every) = self.cfg.auto_fetch {
            if idle && self.last_fetch.elapsed() >= every {
                self.start_fetch(true);
                return;
            }
        }
        if let Some(every) = self.cfg.refresh {
            if idle && self.last_refresh.elapsed() >= every {
                self.refresh();
                if self
                    .last_pr_load
                    .is_none_or(|t| t.elapsed() > Duration::from_secs(60))
                {
                    self.load_prs();
                }
            }
        }
    }

    fn finish_job(&mut self, job: Job, result: Result<Outcome, String>) {
        match result {
            Ok(Outcome::Message(m)) => {
                let m = m
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .to_string();
                self.set_message(format!("✓ {}: {m}", job.title), Color::Green);
                self.refresh();
                self.last_pr_load = None;
                self.load_prs();
            }
            Ok(Outcome::Plan(plan)) => {
                self.mode = Mode::Confirm {
                    action: Action::Submit(plan),
                };
                self.set_message("", Color::Reset);
                self.rerender();
            }
            Ok(Outcome::Prs(prs)) => {
                self.prs = prs;
                self.rerender();
            }
            Err(e) => {
                let mut lines: Vec<Line<'static>> =
                    job.log.iter().map(|l| Line::raw(l.clone())).collect();
                lines.push(Line::raw(""));
                lines.extend(
                    e.lines()
                        .map(|l| Line::styled(l.to_string(), Style::default().fg(Color::Red))),
                );
                self.set_message(format!("✗ {} failed", job.title), Color::Red);
                if !job.background {
                    self.mode = Mode::Text {
                        title: format!("{} failed", job.title),
                        lines,
                        scroll: 0,
                    };
                }
                self.refresh();
            }
        }
    }

    fn start_fetch(&mut self, background: bool) {
        let jj = self.jj.clone();
        let remote = self.cfg.remote.clone();
        let rebase = self.cfg.auto_rebase;
        self.last_fetch = Instant::now();
        let title = if background { "auto-fetch" } else { "fetch" };
        self.job = Some(spawn_job(title, background, move |log| {
            Ok(Outcome::Message(stack::fetch_and_rebase(
                &jj, &remote, rebase, log,
            )?))
        }));
    }

    fn run_op(&mut self, op: Op) {
        let cmds = op.commands(&self.dag);
        let title = op.title(&self.dag);
        let jj = self.jj.clone();
        self.mode = Mode::Normal;
        self.job = Some(spawn_job(title, false, move |log| {
            let mut last = String::new();
            for cmd in cmds {
                log(format!("$ jj {}", shell_join(&cmd)));
                let out = jj.run(&cmd)?;
                last = out.summary();
                for l in last.lines() {
                    log(l.to_string());
                }
            }
            Ok(Outcome::Message(last))
        }));
    }

    fn shipped_commits(&self) -> Vec<String> {
        self.dag
            .order
            .iter()
            .filter(|id| {
                let n = &self.dag.nodes[*id];
                !n.commit.immutable
                    && pr_for(n, &self.prs).is_some_and(|p| p.state == PrState::Merged)
            })
            .cloned()
            .collect()
    }

    // ---- preview -------------------------------------------------------------

    fn current_action_op(&self) -> Option<Op> {
        match &self.mode {
            Mode::Pick { pick, rev, target } => {
                self.dag.order.get(*target).map(|t| pick.build(rev, t))
            }
            Mode::Confirm {
                action: Action::Op(op),
            } => Some(op.clone()),
            _ => None,
        }
    }

    fn update_preview(&mut self) {
        let (key, built): (String, Option<BuiltPreview>) = match &self.mode {
            Mode::Pick { .. }
            | Mode::Confirm {
                action: Action::Op(_),
            } => {
                let op = self.current_action_op().expect("op for pick/confirm");
                let key = format!("{op:?}{}", self.ghosts);
                if self.preview.as_ref().is_some_and(|p| p.key == key) {
                    return;
                }
                let preview = op.preview(&self.dag, self.ghosts);
                let focus = match &op {
                    Op::Rebase { rev, .. }
                    | Op::Abandon { rev }
                    | Op::Edit { rev }
                    | Op::Describe { rev, .. } => Some(rev.clone()),
                    Op::Squash { into, .. } => Some(into.clone()),
                    Op::BookmarkSet { target, .. } => Some(target.clone()),
                    _ => Some(crate::dag::NEW_WC_ID.to_string()),
                };
                let cmds = op
                    .commands(&self.dag)
                    .iter()
                    .map(|c| format!("jj {}", shell_join(c).replace('\n', "⏎")))
                    .collect();
                let title = op.title(&self.dag);
                (key, Some((preview, focus, cmds, title)))
            }
            Mode::Confirm {
                action: Action::Submit(plan),
            } => {
                let key = format!("submit{}", plan.entries.len());
                if self.preview.as_ref().is_some_and(|p| p.key == key) {
                    return;
                }
                let mut d = self.dag.clone();
                for e in &plan.entries {
                    if d.get(&e.commit.commit_id).is_some() {
                        d.annotate(&e.commit.commit_id, Preview::Submit, plan.note_for(e));
                        if e.set_bookmark {
                            if let Some(n) = d.nodes.get_mut(&e.commit.commit_id) {
                                n.commit.bookmarks.push(e.bookmark.clone());
                            }
                        }
                    }
                }
                let focus = plan.entries.last().map(|e| e.commit.commit_id.clone());
                let title = format!("Submit {} PR(s) as a stack", plan.entries.len());
                (
                    key,
                    Some((
                        Ok(d),
                        focus,
                        plan.describe(self.cfg.submit.native_stack),
                        title,
                    )),
                )
            }
            Mode::Confirm {
                action: Action::Ship { pr, commit_id },
            } => {
                let key = format!("ship{}", pr.number);
                if self.preview.as_ref().is_some_and(|p| p.key == key) {
                    return;
                }
                let mut d = self.dag.clone();
                let mut lines = vec![];
                let base: HashSet<String> = d.trunk().map(|t| d.ancestors(&t)).unwrap_or_default();
                let stack = d.ancestors(commit_id);
                let ids: Vec<String> = d
                    .order
                    .iter()
                    .filter(|id| stack.contains(*id))
                    .cloned()
                    .collect();
                for id in ids {
                    if base.contains(&id) {
                        continue;
                    }
                    let n = &d.nodes[&id];
                    if let Some(p) = pr_for(n, &self.prs).filter(|p| p.is_open()) {
                        lines.push(format!("#{} {}", p.number, p.title));
                        let note = format!("merges #{}", p.number);
                        d.annotate(&id, Preview::Ship, note);
                    }
                }
                let title = format!(
                    "Ship PR #{} via merge-async ({}){}",
                    pr.number,
                    self.cfg.merge_method,
                    if lines.len() > 1 {
                        format!(", including {} PRs below it", lines.len() - 1)
                    } else {
                        String::new()
                    }
                );
                (key, Some((Ok(d), Some(commit_id.clone()), lines, title)))
            }
            _ => (String::new(), None),
        };
        self.preview = built.map(|(result, focus, commands, title)| PreviewState {
            key,
            result: result.map(|d| render::render(&d, &self.prs, &HashMap::new())),
            focus,
            commands,
            title,
        });
        self.preview_scroll = 0;
    }

    // ---- commands --------------------------------------------------------------

    fn index_of(&self, id: &str) -> Option<usize> {
        self.dag.order.iter().position(|x| x == id)
    }

    fn start_pick(&mut self, pick: Pick, rev: String, start: Option<String>) {
        let target = start.and_then(|s| self.index_of(&s)).unwrap_or(self.cursor);
        self.mode = Mode::Pick { pick, rev, target };
        self.rerender();
    }

    fn confirm(&mut self, op: Op) {
        // Surface invalid ops right away instead of opening a dead preview.
        if let Err(e) = op.preview(&self.dag, false) {
            self.set_message(format!("✗ {}: {e}", op.title(&self.dag)), Color::Red);
            return;
        }
        self.mode = Mode::Confirm {
            action: Action::Op(op),
        };
        self.rerender();
    }

    fn exec(&mut self, cmd: Cmd) {
        let Some(rev) = self.selected() else { return };
        let mutating = !matches!(
            cmd,
            Cmd::Diff | Cmd::Help | Cmd::Quit | Cmd::OpenPr | Cmd::Refresh | Cmd::ToggleAutoRebase
        );
        if mutating && self.busy() {
            return;
        }
        let first_parent = self.dag.parent_ids(&rev).first().cloned();
        match cmd {
            Cmd::Edit => self.confirm(Op::Edit { rev }),
            Cmd::New => self.confirm(Op::New { rev }),
            Cmd::NewBefore => self.confirm(Op::NewBefore { rev }),
            Cmd::NewAfter => self.confirm(Op::NewAfter { rev }),
            Cmd::Describe => {
                let c = &self.dag.nodes[&rev].commit;
                if c.immutable {
                    self.set_message("✗ can't describe an immutable commit", Color::Red);
                    return;
                }
                self.mode = Mode::Input {
                    prompt: format!(
                        "Describe {} (Enter: preview · Ctrl-E: open $EDITOR)",
                        c.short_change
                    ),
                    buffer: c.title().to_string(),
                    next: InputNext::Describe {
                        body: c.body(),
                        rev,
                    },
                };
            }
            Cmd::Bookmark => {
                let existing = self.dag.nodes[&rev]
                    .commit
                    .bookmarks
                    .first()
                    .cloned()
                    .unwrap_or_default();
                self.mode = Mode::Input {
                    prompt: "Bookmark name (existing one on this commit → pick where to move it)"
                        .into(),
                    buffer: existing,
                    next: InputNext::Bookmark { rev },
                };
            }
            Cmd::Rebase => self.start_pick(
                Pick::Rebase {
                    scope: Scope::Source,
                    placement: Placement::Onto,
                },
                rev,
                first_parent,
            ),
            Cmd::RebaseTrunk => match self.dag.trunk() {
                Some(t) => self.confirm(Op::Rebase {
                    scope: Scope::Branch,
                    placement: Placement::Onto,
                    rev,
                    target: t,
                }),
                None => self.set_message("✗ trunk() is not in view", Color::Red),
            },
            Cmd::Squash => match self.dag.parent_ids(&rev).as_slice() {
                [p] => {
                    let p = p.clone();
                    self.confirm(Op::Squash { from: rev, into: p })
                }
                _ => self.set_message(
                    "✗ squash into parent needs exactly one parent; use S",
                    Color::Red,
                ),
            },
            Cmd::SquashInto => self.start_pick(Pick::SquashInto, rev, first_parent),
            Cmd::Split => {
                if self.dag.nodes[&rev].commit.immutable {
                    self.set_message("✗ can't split an immutable commit", Color::Red);
                } else {
                    self.interactive = Some(vec!["split".into(), "-r".into(), rev]);
                }
            }
            Cmd::Abandon => self.confirm(Op::Abandon { rev }),
            Cmd::AbandonSubtree => self.confirm(Op::AbandonSubtree { rev }),
            Cmd::Duplicate => self.confirm(Op::Duplicate { rev }),
            Cmd::Revert => {
                let wc = self.dag.wc();
                self.start_pick(Pick::RevertOnto, rev, wc)
            }
            Cmd::Diff => match self.jj.diff_text(&rev) {
                Ok(t) => {
                    let title = format!("diff {}", self.short(&rev));
                    self.mode = Mode::Text {
                        title,
                        lines: diff_lines(&t),
                        scroll: 0,
                    };
                }
                Err(e) => self.set_message(format!("✗ {e:#}"), Color::Red),
            },
            Cmd::Interdiff => self.interdiff(&rev),
            Cmd::Submit => self.submit(&rev),
            Cmd::Ship => self.ship(&rev),
            Cmd::OpenPr => match pr_for(&self.dag.nodes[&rev], &self.prs) {
                Some(pr) => {
                    let url = pr.url.clone();
                    match open_in_browser(&url) {
                        Ok(()) => self.set_message(format!("opened {url}"), Color::Green),
                        Err(e) => self.set_message(format!("✗ {e:#}"), Color::Red),
                    }
                }
                None => self.set_message("no PR for this commit", Color::Yellow),
            },
            Cmd::Cleanup => {
                let shipped = self.shipped_commits();
                match (shipped.is_empty(), self.dag.trunk()) {
                    (true, _) => self.set_message(
                        "no commits with merged PRs in view (fetch first?)",
                        Color::Yellow,
                    ),
                    (false, None) => self.set_message("✗ trunk() is not in view", Color::Red),
                    (false, Some(t)) => self.confirm(Op::Cleanup { shipped, trunk: t }),
                }
            }
            Cmd::Fetch => self.start_fetch(false),
            Cmd::Push => {
                let op = Op::Push {
                    revset: format!("trunk()..({rev})"),
                    remote: self.cfg.remote.clone(),
                };
                self.run_op(op);
            }
            Cmd::Undo => self.run_op(Op::Undo),
            Cmd::Redo => self.run_op(Op::Redo),
            Cmd::Refresh => {
                self.refresh();
                self.last_pr_load = None;
                self.load_prs();
                self.set_message("refreshed", Color::Green);
            }
            Cmd::ToggleAutoRebase => {
                self.cfg.auto_rebase = !self.cfg.auto_rebase;
                let state = if self.cfg.auto_rebase { "on" } else { "off" };
                self.set_message(format!("auto-rebase after fetch: {state}"), Color::Cyan);
            }
            Cmd::Help => self.mode = Mode::Help { scroll: 0 },
            Cmd::Quit => self.quit = true,
        }
    }

    fn submit(&mut self, rev: &str) {
        let Some(gh) = self.gh.clone() else {
            let err = self.gh_error.clone().unwrap_or_default();
            self.set_message(format!("✗ GitHub unavailable: {err}"), Color::Red);
            return;
        };
        let jj = self.jj.clone();
        let rev = rev.to_string();
        let trunk = self.trunk_branch.clone();
        self.job = Some(spawn_job("planning PR stack", false, move |_| {
            let commits = stack::stack_commits(&jj, &rev)?;
            let mut branches: Vec<String> =
                commits.iter().flat_map(|c| c.bookmarks.clone()).collect();
            branches.extend(
                commits
                    .iter()
                    .map(|c| stack::default_bookmark(&c.change_id)),
            );
            let prs = gh.prs_for_branches(&branches)?;
            Ok(Outcome::Plan(Box::new(stack::plan(commits, &prs, &trunk))))
        }));
    }

    fn ship(&mut self, rev: &str) {
        let n = &self.dag.nodes[rev];
        match pr_for(n, &self.prs) {
            Some(pr) if pr.is_open() => {
                self.mode = Mode::Confirm {
                    action: Action::Ship {
                        pr: Box::new(pr.clone()),
                        commit_id: rev.to_string(),
                    },
                };
                self.rerender();
            }
            Some(pr) => self.set_message(format!("PR #{} is not open", pr.number), Color::Yellow),
            None => self.set_message("no PR for this commit (P creates one)", Color::Yellow),
        }
    }

    /// Pro: re-review only what changed — against the last pushed version,
    /// the previous local version, or the commit a reviewer last saw.
    fn interdiff(&mut self, rev: &str) {
        let n = self.dag.nodes[rev].clone();
        let mut items: Vec<(String, Choice)> = vec![];
        let to = n.commit.commit_id.clone();
        for b in &n.commit.bookmarks {
            let remote = format!("{b}@{}", self.cfg.remote);
            if !n.commit.remote_bookmarks.contains(&remote)
                && self.jj.exists(&format!("\"{b}\"@{}", self.cfg.remote))
            {
                items.push((
                    format!("since last push ({remote})"),
                    Choice::Interdiff {
                        from: format!("\"{b}\"@{}", self.cfg.remote),
                        to: to.clone(),
                        title: format!("interdiff {remote} → local"),
                    },
                ));
            }
        }
        if let Some(pr) = pr_for(&n, &self.prs) {
            let mut seen = HashSet::new();
            for r in &pr.reviews {
                if r.oid != to && seen.insert(r.oid.clone()) && self.jj.exists(&r.oid) {
                    items.push((
                        format!(
                            "since {}'s review ({}, {})",
                            r.login,
                            &r.oid[..8.min(r.oid.len())],
                            r.state.to_lowercase()
                        ),
                        Choice::Interdiff {
                            from: r.oid.clone(),
                            to: to.clone(),
                            title: format!("interdiff since {}'s review", r.login),
                        },
                    ));
                }
            }
        }
        if let Ok(preds) = self.jj.predecessors(rev) {
            if let Some(p) = preds.first() {
                items.push((
                    "since previous local version (evolog)".into(),
                    Choice::Interdiff {
                        from: p.clone(),
                        to: to.clone(),
                        title: "interdiff vs previous version".into(),
                    },
                ));
            }
        }
        match items.len() {
            0 => self.set_message(
                "nothing to compare: no earlier pushed, reviewed or evolved version",
                Color::Yellow,
            ),
            1 => {
                let (_, c) = items.remove(0);
                self.choose(c);
            }
            _ => {
                let mut state = ListState::default();
                state.select(Some(0));
                self.mode = Mode::Choose {
                    title: format!("Interdiff {}", n.commit.short_change),
                    items,
                    state,
                };
            }
        }
    }

    fn choose(&mut self, choice: Choice) {
        match choice {
            Choice::Interdiff { from, to, title } => match self.jj.interdiff_text(&from, &to) {
                Ok(t) => {
                    let body = if !t.contains("diff --git") {
                        format!("{t}\n(no changes besides rebasing — nothing new to review)")
                    } else {
                        t
                    };
                    self.mode = Mode::Text {
                        title,
                        lines: diff_lines(&body),
                        scroll: 0,
                    };
                }
                Err(e) => {
                    self.mode = Mode::Normal;
                    self.set_message(format!("✗ {e:#}"), Color::Red)
                }
            },
        }
    }

    fn confirm_action(&mut self) {
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        let action = match mode {
            Mode::Confirm { action } => action,
            Mode::Pick { pick, rev, target } => match self.dag.order.get(target) {
                Some(t) => Action::Op(pick.build(&rev, t)),
                None => return,
            },
            other => {
                self.mode = other;
                return;
            }
        };
        if let Some(Err(e)) = self.preview.as_ref().map(|p| &p.result) {
            self.set_message(format!("✗ can't run: {e}"), Color::Red);
            self.preview = None;
            self.rerender();
            return;
        }
        self.preview = None;
        match action {
            Action::Op(op) => self.run_op(op),
            Action::Submit(plan) => {
                let gh = self.gh.clone().expect("gh present for submit");
                let jj = self.jj.clone();
                let opts = self.cfg.submit.clone();
                self.job = Some(spawn_job("submit PR stack", false, move |log| {
                    let numbers = stack::execute(&jj, &gh, &plan, &opts, log)?;
                    let list: Vec<String> = numbers.iter().map(|n| format!("#{n}")).collect();
                    Ok(Outcome::Message(format!("PRs {}", list.join(" ← "))))
                }));
            }
            Action::Ship { pr, commit_id } => {
                let gh = self.gh.clone().expect("gh present for ship");
                let jj = self.jj.clone();
                let commit = self.dag.nodes[&commit_id].commit.clone();
                let method = self.cfg.merge_method.clone();
                let remote = self.cfg.remote.clone();
                self.job = Some(spawn_job(
                    format!("ship #{}", pr.number),
                    false,
                    move |log| {
                        Ok(Outcome::Message(stack::ship(
                            &jj, &gh, &pr, &commit, &method, &remote, log,
                        )?))
                    },
                ));
            }
        }
        self.rerender();
    }

    // ---- keys -------------------------------------------------------------------

    fn move_cursor(&mut self, delta: isize) {
        let len = self.dag.order.len() as isize;
        if len == 0 {
            return;
        }
        self.cursor = (self.cursor as isize + delta).clamp(0, len - 1) as usize;
        self.rerender();
    }

    fn on_key(&mut self, k: KeyEvent) {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        let mode = std::mem::replace(&mut self.mode, Mode::Normal);
        match mode {
            Mode::Normal => {
                self.mode = Mode::Normal;
                self.normal_key(k);
            }
            Mode::Menu { mut state } => match k.code {
                KeyCode::Esc | KeyCode::Char('q') => {}
                KeyCode::Down | KeyCode::Char('j') => {
                    state.select_next();
                    self.mode = Mode::Menu { state };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.select_previous();
                    self.mode = Mode::Menu { state };
                }
                KeyCode::Enter => {
                    let i = state.selected().unwrap_or(0).min(COMMANDS.len() - 1);
                    self.exec(COMMANDS[i].2);
                }
                KeyCode::Char(c) => match COMMANDS.iter().find(|x| x.1 == c) {
                    Some(x) => self.exec(x.2),
                    None => self.mode = Mode::Menu { state },
                },
                _ => self.mode = Mode::Menu { state },
            },
            Mode::Pick {
                mut pick,
                rev,
                mut target,
            } => {
                let len = self.dag.order.len();
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.preview = None;
                        self.rerender();
                        return;
                    }
                    KeyCode::Enter => {
                        self.mode = Mode::Pick { pick, rev, target };
                        self.update_preview();
                        self.confirm_action();
                        return;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        target = (target + 1).min(len.saturating_sub(1))
                    }
                    KeyCode::Up | KeyCode::Char('k') => target = target.saturating_sub(1),
                    KeyCode::PageDown => target = (target + 10).min(len.saturating_sub(1)),
                    KeyCode::PageUp => target = target.saturating_sub(10),
                    KeyCode::Home | KeyCode::Char('g') => target = 0,
                    KeyCode::End | KeyCode::Char('G') => target = len.saturating_sub(1),
                    KeyCode::Char('@') => {
                        target = self
                            .dag
                            .wc()
                            .and_then(|w| self.index_of(&w))
                            .unwrap_or(target)
                    }
                    KeyCode::Char('t') => {
                        target = self
                            .dag
                            .trunk()
                            .and_then(|w| self.index_of(&w))
                            .unwrap_or(target)
                    }
                    KeyCode::Tab => {
                        if let Pick::Rebase { scope, placement } = &mut pick {
                            *placement = placement.next();
                            if *placement != Placement::Onto {
                                *scope = Scope::Revision;
                            }
                        }
                    }
                    KeyCode::Char('s') => {
                        if let Pick::Rebase { scope, placement } = &mut pick {
                            *scope = scope.next();
                            if *scope != Scope::Revision {
                                *placement = Placement::Onto;
                            }
                        }
                    }
                    KeyCode::Char('h') => self.ghosts = !self.ghosts,
                    KeyCode::Char('J') => self.preview_scroll += 1,
                    KeyCode::Char('K') => {
                        self.preview_scroll = self.preview_scroll.saturating_sub(1)
                    }
                    _ => {}
                }
                self.mode = Mode::Pick { pick, rev, target };
                self.rerender();
            }
            Mode::Confirm { action } => match k.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.mode = Mode::Confirm { action };
                    self.confirm_action();
                }
                KeyCode::Char('h') => {
                    self.ghosts = !self.ghosts;
                    self.mode = Mode::Confirm { action };
                    self.rerender();
                }
                KeyCode::Char('J') => {
                    self.preview_scroll += 1;
                    self.mode = Mode::Confirm { action };
                }
                KeyCode::Char('K') => {
                    self.preview_scroll = self.preview_scroll.saturating_sub(1);
                    self.mode = Mode::Confirm { action };
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                    self.preview = None;
                    self.rerender();
                }
                _ => self.mode = Mode::Confirm { action },
            },
            Mode::Input {
                prompt,
                mut buffer,
                next,
            } => match k.code {
                KeyCode::Esc => {}
                KeyCode::Enter => self.submit_input(buffer, next),
                KeyCode::Backspace => {
                    buffer.pop();
                    self.mode = Mode::Input {
                        prompt,
                        buffer,
                        next,
                    };
                }
                KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.mode = Mode::Input {
                        prompt,
                        buffer: String::new(),
                        next,
                    };
                }
                KeyCode::Char('e') if k.modifiers.contains(KeyModifiers::CONTROL) => match next {
                    InputNext::Describe { rev, .. } => {
                        self.interactive = Some(vec!["describe".into(), rev]);
                    }
                    other => {
                        self.mode = Mode::Input {
                            prompt,
                            buffer,
                            next: other,
                        }
                    }
                },
                KeyCode::Char(c) => {
                    buffer.push(c);
                    self.mode = Mode::Input {
                        prompt,
                        buffer,
                        next,
                    };
                }
                _ => {
                    self.mode = Mode::Input {
                        prompt,
                        buffer,
                        next,
                    }
                }
            },
            Mode::Choose {
                title,
                items,
                mut state,
            } => match k.code {
                KeyCode::Esc | KeyCode::Char('q') => {}
                KeyCode::Down | KeyCode::Char('j') => {
                    state.select_next();
                    self.mode = Mode::Choose {
                        title,
                        items,
                        state,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.select_previous();
                    self.mode = Mode::Choose {
                        title,
                        items,
                        state,
                    };
                }
                KeyCode::Enter => {
                    let i = state.selected().unwrap_or(0).min(items.len() - 1);
                    self.choose(items[i].1.clone());
                }
                _ => {
                    self.mode = Mode::Choose {
                        title,
                        items,
                        state,
                    }
                }
            },
            Mode::Text {
                title,
                lines,
                mut scroll,
            } => {
                let max = lines.len().saturating_sub(1);
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => return,
                    KeyCode::Down | KeyCode::Char('j') => scroll = (scroll + 1).min(max),
                    KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('d') => {
                        scroll = (scroll + 20).min(max)
                    }
                    KeyCode::PageUp | KeyCode::Char('b') | KeyCode::Char('u') => {
                        scroll = scroll.saturating_sub(20)
                    }
                    KeyCode::Char('g') | KeyCode::Home => scroll = 0,
                    KeyCode::Char('G') | KeyCode::End => scroll = max,
                    _ => {}
                }
                self.mode = Mode::Text {
                    title,
                    lines,
                    scroll,
                };
            }
            Mode::Help { mut scroll } => match k.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    scroll += 1;
                    self.mode = Mode::Help { scroll };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    scroll = scroll.saturating_sub(1);
                    self.mode = Mode::Help { scroll };
                }
                _ => {}
            },
        }
    }

    fn normal_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Char('d') if ctrl => self.move_cursor(10),
            KeyCode::Char('u') if ctrl => self.move_cursor(-10),
            KeyCode::PageDown => self.move_cursor(10),
            KeyCode::PageUp => self.move_cursor(-10),
            KeyCode::Home | KeyCode::Char('g') => {
                self.move_cursor(-(self.dag.order.len() as isize))
            }
            KeyCode::End | KeyCode::Char('G') => self.move_cursor(self.dag.order.len() as isize),
            KeyCode::Char('@') => {
                if let Some(i) = self.dag.wc().and_then(|w| self.index_of(&w)) {
                    self.cursor = i;
                    self.rerender();
                }
            }
            KeyCode::Enter => {
                let mut state = ListState::default();
                state.select(Some(0));
                self.mode = Mode::Menu { state };
            }
            KeyCode::Char(' ') => self.exec(Cmd::Diff),
            KeyCode::Esc => self.set_message("", Color::Reset),
            KeyCode::Char('h') => {
                self.ghosts = !self.ghosts;
                let state = if self.ghosts { "shown" } else { "hidden" };
                self.set_message(format!("old positions in previews: {state}"), Color::Cyan);
            }
            KeyCode::Char('L') => {
                if let Some(job) = &self.job {
                    let lines = job.log.iter().map(|l| Line::raw(l.clone())).collect();
                    self.mode = Mode::Text {
                        title: format!("{} (running)", job.title),
                        lines,
                        scroll: 0,
                    };
                }
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(x) = COMMANDS.iter().find(|x| x.1 == c) {
                    self.exec(x.2);
                }
            }
            _ => {}
        }
    }

    fn submit_input(&mut self, buffer: String, next: InputNext) {
        match next {
            InputNext::Describe { rev, body } => {
                let title = buffer.trim();
                let message = if body.is_empty() {
                    title.to_string()
                } else {
                    format!("{title}\n\n{body}")
                };
                self.confirm(Op::Describe { rev, message });
            }
            InputNext::Bookmark { rev } => {
                let name = buffer.trim().to_string();
                if name.is_empty() || name.contains(char::is_whitespace) {
                    self.set_message("✗ invalid bookmark name", Color::Red);
                    return;
                }
                if self.dag.nodes[&rev].commit.bookmarks.contains(&name) {
                    self.start_pick(Pick::BookmarkMove { name }, rev.clone(), Some(rev));
                } else {
                    self.confirm(Op::BookmarkSet { name, target: rev });
                }
            }
        }
    }

    // ---- drawing -----------------------------------------------------------------

    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let [main, status] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(area);
        let previewing = matches!(self.mode, Mode::Pick { .. } | Mode::Confirm { .. });
        if previewing {
            self.draw_preview_layout(f, main);
        } else {
            let wide = main.width >= 110;
            let (graph_area, detail_area) = if wide {
                let [g, d] =
                    Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
                        .areas(main);
                (g, d)
            } else {
                let [g, d] =
                    Layout::vertical([Constraint::Min(5), Constraint::Length(9)]).areas(main);
                (g, d)
            };
            let focus = self.selected();
            let title = format!(" {} ", self.root);
            let scroll = scroll_for(
                &self.rendered,
                focus.as_deref(),
                graph_area.height.saturating_sub(2) as usize,
                self.scroll,
            );
            self.scroll = scroll;
            let p = Paragraph::new(self.rendered.lines.clone())
                .block(Block::default().borders(Borders::ALL).title(title))
                .scroll((scroll as u16, 0));
            f.render_widget(p, graph_area);
            self.draw_details(f, detail_area);
        }
        self.draw_status(f, status);

        let menu_title = self
            .selected()
            .map(|id| format!(" {} — pick a command ", self.dag.short(&id)))
            .unwrap_or_default();
        match &mut self.mode {
            Mode::Menu { state } => {
                let items: Vec<ListItem> = COMMANDS
                    .iter()
                    .map(|(group, key, _, label)| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!(" {key} "),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(label.to_string()),
                            Span::styled(
                                format!("  {group}"),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]))
                    })
                    .collect();
                let rect = centered(area, 56, (COMMANDS.len() + 2) as u16);
                f.render_widget(Clear, rect);
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(menu_title))
                    .highlight_style(
                        Style::default()
                            .bg(Color::Rgb(40, 52, 80))
                            .add_modifier(Modifier::BOLD),
                    );
                f.render_stateful_widget(list, rect, state);
            }
            Mode::Choose {
                title,
                items,
                state,
            } => {
                let list_items: Vec<ListItem> = items
                    .iter()
                    .map(|(l, _)| ListItem::new(format!(" {l}")))
                    .collect();
                let rect = centered(area, 70, (items.len() + 2) as u16);
                f.render_widget(Clear, rect);
                let list = List::new(list_items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" {title} ")),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Rgb(40, 52, 80))
                            .add_modifier(Modifier::BOLD),
                    );
                f.render_stateful_widget(list, rect, state);
            }
            Mode::Input { prompt, buffer, .. } => {
                let rect = centered(area, 80, 3);
                f.render_widget(Clear, rect);
                let p = Paragraph::new(Line::from(vec![
                    Span::raw(buffer.clone()),
                    Span::styled("▏", Style::default().fg(Color::Yellow)),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {prompt} ")),
                );
                f.render_widget(p, rect);
            }
            Mode::Text {
                title,
                lines,
                scroll,
            } => {
                let rect = inset(area, 2, 1);
                f.render_widget(Clear, rect);
                let p = Paragraph::new(lines.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" {title} — j/k scroll · q close ")),
                    )
                    .scroll((*scroll as u16, 0));
                f.render_widget(p, rect);
            }
            Mode::Help { scroll } => {
                let rect = inset(area, 4, 1);
                f.render_widget(Clear, rect);
                let p = Paragraph::new(help_lines())
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" jj-helper — keys (any other key closes) "),
                    )
                    .scroll((*scroll as u16, 0));
                f.render_widget(p, rect);
            }
            _ => {}
        }
    }

    fn draw_preview_layout(&mut self, f: &mut Frame, area: Rect) {
        let shown = self
            .preview
            .as_ref()
            .map(|p| p.commands.len().min(10))
            .unwrap_or(0);
        let [panes, info] =
            Layout::vertical([Constraint::Min(5), Constraint::Length(shown as u16 + 4)])
                .areas(area);
        let dir = if panes.width >= 120 {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        let [before, after] = Layout::default()
            .direction(dir)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(panes);

        let focus = match &self.mode {
            Mode::Pick { target, .. } => self.dag.order.get(*target).cloned(),
            _ => self.selected(),
        };
        let h = before.height.saturating_sub(2) as usize;
        let scroll = scroll_for(&self.rendered, focus.as_deref(), h, self.scroll);
        self.scroll = scroll;
        let before_title = match &self.mode {
            Mode::Pick { .. } => " Before — move to the destination (j/k, @, t) ",
            _ => " Before ",
        };
        f.render_widget(
            Paragraph::new(self.rendered.lines.clone())
                .block(Block::default().borders(Borders::ALL).title(before_title))
                .scroll((scroll as u16, 0)),
            before,
        );

        let Some(preview) = &self.preview else { return };
        let ah = after.height.saturating_sub(2) as usize;
        match &preview.result {
            Ok(r) => {
                let base = scroll_for(r, preview.focus.as_deref(), ah, 0);
                let s = base + self.preview_scroll;
                f.render_widget(
                    Paragraph::new(r.lines.clone())
                        .block(
                            Block::default().borders(Borders::ALL).title(Span::styled(
                                " After (preview) ",
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            )),
                        )
                        .scroll((s as u16, 0)),
                    after,
                );
            }
            Err(e) => {
                f.render_widget(
                    Paragraph::new(vec![
                        Line::raw(""),
                        Line::styled(format!("  ✗ {e}"), Style::default().fg(Color::Red)),
                    ])
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(Span::styled(" After ", Style::default().fg(Color::Red))),
                    ),
                    after,
                );
            }
        }

        let mut lines = vec![];
        let mode_label = match &self.mode {
            Mode::Pick { pick, .. } => pick.label(),
            _ => String::new(),
        };
        lines.push(Line::from(vec![
            Span::styled(
                preview.title.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(mode_label, Style::default().fg(Color::Cyan)),
        ]));
        for c in preview.commands.iter().take(10) {
            lines.push(Line::styled(
                format!("  {c}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let hints = match &self.mode {
            Mode::Pick { pick: Pick::Rebase { .. }, .. } => {
                "Enter run · Tab onto/after/before · s scope (-s/-r/-b) · h old positions · J/K scroll after · Esc cancel"
            }
            Mode::Pick { .. } => "Enter run · j/k choose destination · h old positions · Esc cancel",
            _ => "Enter/y run · h old positions · J/K scroll after · Esc/n cancel",
        };
        let ok = preview.result.is_ok();
        lines.push(Line::styled(
            hints,
            Style::default().fg(if ok { Color::Yellow } else { Color::Red }),
        ));
        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::TOP)),
            info,
        );
    }

    fn draw_details(&mut self, f: &mut Frame, area: Rect) {
        let Some(id) = self.selected() else { return };
        let n = self.dag.nodes[&id].clone();
        let c = &n.commit;
        let mut lines: Vec<Line<'static>> = vec![];
        lines.push(Line::from(vec![
            Span::styled("Change ", Style::default().fg(Color::DarkGray)),
            Span::styled(c.change_id.clone(), Style::default().fg(Color::Magenta)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Commit ", Style::default().fg(Color::DarkGray)),
            Span::styled(c.commit_id.clone(), Style::default().fg(Color::Blue)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Author ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} <{}> · {}",
                c.author_name,
                c.author_email,
                relative_time(&c.timestamp)
            )),
        ]));
        let mut flags = vec![];
        if c.is_wc {
            flags.push("working copy");
        }
        if c.immutable {
            flags.push("immutable");
        }
        if c.empty {
            flags.push("empty");
        }
        if c.conflict {
            flags.push("CONFLICT");
        }
        if c.divergent {
            flags.push("divergent");
        }
        if !flags.is_empty() {
            lines.push(Line::styled(
                flags.join(" · "),
                Style::default().fg(Color::Yellow),
            ));
        }
        if !c.bookmarks.is_empty() || !c.remote_bookmarks.is_empty() {
            let mut all = c.bookmarks.clone();
            all.extend(c.remote_bookmarks.iter().cloned());
            lines.push(Line::from(vec![
                Span::styled("Bookmarks ", Style::default().fg(Color::DarkGray)),
                Span::styled(all.join(" "), Style::default().fg(Color::Magenta)),
            ]));
        }
        if let Some(pr) = pr_for(&n, &self.prs) {
            lines.push(Line::raw(""));
            let mut spans = vec![Span::styled("PR ", Style::default().fg(Color::DarkGray))];
            spans.extend(pr_badge(pr));
            lines.push(Line::from(spans));
            lines.push(Line::raw(pr.title.clone()).bold());
            lines.push(Line::styled(
                format!("{} ← {}", pr.base_ref, pr.head_ref),
                Style::default().fg(Color::DarkGray),
            ));
            if let Some(s) = &pr.stack {
                lines.push(Line::styled(
                    format!(
                        "GitHub stack #{}: position {} of {}",
                        s.number, s.position, s.size
                    ),
                    Style::default().fg(Color::Blue),
                ));
            }
            if pr.is_open() && pr.head_oid != c.commit_id {
                lines.push(Line::styled(
                    "local commit differs from PR head — P to push",
                    Style::default().fg(Color::Yellow),
                ));
            }
            lines.push(Line::styled(
                pr.url.clone(),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::raw(""));
        if c.description.trim().is_empty() {
            lines.push(Line::styled(
                "(no description set)",
                Style::default().fg(Color::Yellow),
            ));
        } else {
            for l in c.description.trim_end().lines() {
                lines.push(Line::raw(l.to_string()));
            }
        }
        if !id.contains(':') {
            if !self.files_cache.contains_key(&id) {
                let summary = self
                    .jj
                    .run(&["diff", "--ignore-working-copy", "--summary", "-r", &id])
                    .map(|o| o.stdout)
                    .unwrap_or_default();
                self.files_cache.insert(id.clone(), summary);
            }
            let files = &self.files_cache[&id];
            if !files.trim().is_empty() {
                lines.push(Line::raw(""));
                for l in files.lines().take(40) {
                    let color = match l.chars().next() {
                        Some('A') => Color::Green,
                        Some('D') => Color::Red,
                        Some('M') => Color::Yellow,
                        _ => Color::Cyan,
                    };
                    lines.push(Line::styled(l.to_string(), Style::default().fg(color)));
                }
            }
        }
        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(" Details ")),
            area,
        );
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        let [l1, l2] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        let mut spans = vec![];
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        for job in [&self.job, &self.pr_job].into_iter().flatten() {
            let i = (job.started.elapsed().as_millis() / 100) as usize % spinner.len();
            let last = job.log.last().cloned().unwrap_or_default();
            spans.push(Span::styled(
                format!("{} {} {last}  ", spinner[i], job.title),
                Style::default().fg(Color::Cyan),
            ));
        }
        if spans.is_empty() {
            spans.push(Span::styled(
                self.message.0.clone(),
                Style::default().fg(self.message.1),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), l1);
        let auto = match self.cfg.auto_fetch {
            Some(d) => format!(
                "auto-fetch {}m{}",
                d.as_secs() / 60,
                if self.cfg.auto_rebase { "+rebase" } else { "" }
            ),
            None => "auto-fetch off".into(),
        };
        let gh = if self.gh.is_some() {
            format!("{} PRs", self.prs.len())
        } else {
            "no GitHub".into()
        };
        let hints = Line::from(vec![
            Span::styled(" Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" commands  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" rebase  "),
            Span::styled("s/S", Style::default().fg(Color::Yellow)),
            Span::raw(" squash  "),
            Span::styled("P", Style::default().fg(Color::Yellow)),
            Span::raw(" PR stack  "),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw(" help  "),
            Span::styled(
                format!(
                    "│ {gh} · {auto} · fetched {} ago",
                    fmt_dur(self.last_fetch.elapsed())
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        f.render_widget(Paragraph::new(hints), l2);
    }
}

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else {
        format!("{}m", s / 60)
    }
}

/// Keeps the focused node (and its detail line) visible.
fn scroll_for(r: &Rendered, focus: Option<&str>, height: usize, current: usize) -> usize {
    let Some(line) = focus.and_then(|f| r.node_line.get(f)).copied() else {
        return current;
    };
    if height == 0 {
        return 0;
    }
    let max = r.lines.len().saturating_sub(height);
    let s = if line < current + 1 {
        line.saturating_sub(2)
    } else if line + 2 >= current + height {
        (line + 3).saturating_sub(height)
    } else {
        current
    };
    s.min(max)
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

fn inset(area: Rect, dx: u16, dy: u16) -> Rect {
    Rect {
        x: area.x + dx.min(area.width / 4),
        y: area.y + dy.min(area.height / 4),
        width: area.width.saturating_sub(2 * dx.min(area.width / 4)),
        height: area.height.saturating_sub(2 * dy.min(area.height / 4)),
    }
}

fn help_lines() -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled("Select a commit (j/k, g/G, @), then press a command key — or Enter for the menu.", Style::default().bold()),
        Line::raw("Commands that change the graph open a Before / After preview; nothing runs until you press Enter."),
        Line::raw("Commands with a destination (rebase, squash into, revert, move bookmark) let you move a target cursor"),
        Line::raw("while the After pane updates live. Space shows the diff, h toggles old positions in previews."),
        Line::raw(""),
    ];
    let mut group = "";
    for (g, key, _, label) in COMMANDS {
        if *g != group {
            group = g;
            lines.push(Line::styled(
                g.to_string(),
                Style::default().fg(Color::Cyan).bold(),
            ));
        }
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<3}"), Style::default().fg(Color::Yellow)),
            Span::raw(label.to_string()),
        ]));
    }
    lines.extend([
        Line::raw(""),
        Line::styled("Preview legend", Style::default().fg(Color::Cyan).bold()),
        Line::styled("  cyan      moved / carried along", Style::default().fg(Color::Cyan)),
        Line::styled("  ◌ gray    old position", Style::default().fg(Color::DarkGray)),
        Line::styled("  ⊘ red     abandoned / folded away", Style::default().fg(Color::Red)),
        Line::styled("  green     created / receives changes / @ destination", Style::default().fg(Color::Green)),
        Line::styled("  magenta   gets a PR", Style::default().fg(Color::Magenta)),
        Line::raw(""),
        Line::styled("PR badges", Style::default().fg(Color::Cyan).bold()),
        Line::raw("  #12 open · ✓approved / ✗changes / ◑reviewed · ● CI (green/yellow/red) · ⧉2/3 GitHub stack position"),
    ]);
    lines
}
