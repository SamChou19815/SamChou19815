//! Interactive TUI for `sam knowledge-graph`, in three tabs:
//!
//! - **Graph** — the node list plus a detail pane; add/rename/delete nodes and
//!   toggle undirected links between them.
//! - **Node** — the selected node's markdown note: a lightly styled read view,
//!   and a plain multi-line editor behind `e`.
//! - **Visualize** — the whole graph drawn on a braille canvas with a
//!   force-directed (Fruchterman–Reingold) layout, so clusters and hubs are
//!   visible at a glance.
//!
//! Every mutation persists to disk immediately, so quitting can never lose
//! structural changes; an open editor commits on save/exit (and on quit).

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::{DefaultTerminal, Frame};

use super::knowledge_graph::{save, Graph};

const TAB_TITLES: [&str; 3] = ["Graph", "Node", "Visualize"];
const GRAPH_TAB: usize = 0;
const NODE_TAB: usize = 1;
const VIZ_TAB: usize = 2;

/// Longest node title shown on the canvas before truncation with `…`.
const VIZ_LABEL_CHARS: usize = 14;

/// A minimal multi-line text editor over the note's lines. Cursor and column
/// positions are in *chars* (not bytes), matching how the terminal cursor
/// moves.
struct Editor {
    lines: Vec<String>,
    row: usize,
    col: usize,
    scroll: usize,
}

impl Editor {
    fn from_text(text: &str) -> Self {
        Self {
            // `"".split('\n')` yields one empty line, so `lines` is never empty.
            lines: text.split('\n').map(str::to_string).collect(),
            row: 0,
            col: 0,
            scroll: 0,
        }
    }

    fn text(&self) -> String {
        self.lines.join("\n")
    }

    fn line_len(&self) -> usize {
        self.lines[self.row].chars().count()
    }

    fn insert_char(&mut self, c: char) {
        let at = byte_index(&self.lines[self.row], self.col);
        self.lines[self.row].insert(at, c);
        self.col += 1;
    }

    fn insert_newline(&mut self) {
        let at = byte_index(&self.lines[self.row], self.col);
        let rest = self.lines[self.row].split_off(at);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
    }

    fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            let at = byte_index(&self.lines[self.row], self.col);
            self.lines[self.row].remove(at);
        } else if self.row > 0 {
            // Join this line onto the previous one.
            let current = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len();
            self.lines[self.row].push_str(&current);
        }
    }

    fn delete(&mut self) {
        if self.col < self.line_len() {
            let at = byte_index(&self.lines[self.row], self.col);
            self.lines[self.row].remove(at);
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    fn move_vertical(&mut self, delta: isize) {
        let last = self.lines.len() as isize - 1;
        self.row = (self.row as isize + delta).clamp(0, last) as usize;
        self.col = self.col.min(self.line_len());
    }
}

/// What key presses currently mean. `Browse` covers all three tabs; the other
/// variants are modal overlays (or, for `EditBody`, the node editor).
enum Mode {
    Browse,
    /// Add (`editing: None`) or rename (`Some(id)`) a node.
    TitleForm {
        editing: Option<u64>,
        value: String,
        cursor: usize,
    },
    /// Pick another node and toggle the link to it. Stays open so several
    /// links can be toggled in one visit.
    LinkPicker {
        others: Vec<u64>,
        state: ListState,
    },
    ConfirmDelete {
        id: u64,
    },
    EditBody {
        editor: Editor,
    },
}

/// Screen regions recorded during each draw, so mouse events can be
/// hit-tested against what is actually on screen (the terminal only reports
/// cell coordinates).
#[derive(Default)]
struct HitAreas {
    /// The tabs row: its y coordinate and each tab's `[start, end)` x range.
    tabs: Option<(u16, Vec<(u16, u16)>)>,
    /// Inner area of the graph tab's node list (inside the border).
    node_list: Option<Rect>,
    /// The visualize tab's canvas area plus its data-space x/y bounds.
    canvas: Option<(Rect, [f64; 2], [f64; 2])>,
    /// Inner area of the link-picker popup.
    picker: Option<Rect>,
}

/// Per-tab x ranges (`[start, end)`) of the tab titles as the `Tabs` widget
/// lays them out: one cell of padding either side of each title, one divider
/// cell between tabs.
fn tab_ranges(origin: u16) -> Vec<(u16, u16)> {
    let mut ranges = Vec::new();
    let mut x = origin;
    for title in TAB_TITLES {
        let width = title.len() as u16 + 2;
        ranges.push((x, x + width));
        x += width + 1;
    }
    ranges
}

struct App {
    graph: Graph,
    path: PathBuf,
    tab: usize,
    /// Selected node, shared across all tabs (the graph list, the node view,
    /// and the highlighted node on the canvas).
    list_state: ListState,
    /// Scroll offset of the node tab's read view.
    body_scroll: u16,
    mode: Mode,
    /// Cached force-layout positions, index-aligned with `graph.nodes`;
    /// invalidated by any structural mutation.
    layout: Option<Vec<(f64, f64)>>,
    /// Re-seeded by `r` on the visualize tab to shake out a fresh layout.
    layout_seed: u64,
    /// One-shot message shown in the status line: `(text, is_error)`.
    status: Option<(String, bool)>,
    hit: HitAreas,
    quit: bool,
}

pub fn run(graph: Graph, path: PathBuf) -> Result<()> {
    let mut app = App::new(graph, path);
    let mut terminal = ratatui::init();
    // Best-effort: the TUI is fully keyboard-driven, so a terminal that
    // rejects mouse capture is not an error.
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    let result = app.run(&mut terminal);
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

impl App {
    fn new(graph: Graph, path: PathBuf) -> Self {
        let mut list_state = ListState::default();
        if !graph.nodes.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            graph,
            path,
            tab: GRAPH_TAB,
            list_state,
            body_scroll: 0,
            mode: Mode::Browse,
            layout: None,
            layout_seed: 0x5EED,
            status: None,
            hit: HitAreas::default(),
            quit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                Event::Mouse(mouse) => self.on_mouse(mouse),
                _ => {}
            }
        }
        Ok(())
    }

    fn selected_index(&self) -> Option<usize> {
        self.list_state
            .selected()
            .filter(|index| *index < self.graph.nodes.len())
    }

    fn selected_id(&self) -> Option<u64> {
        self.selected_index()
            .map(|index| self.graph.nodes[index].id)
    }

    /// Persist the graph, reporting failure in the status line (the in-memory
    /// state stays authoritative either way).
    fn persist(&mut self) {
        if let Err(err) = save(&self.path, &self.graph) {
            self.status = Some((err.to_string(), true));
        }
    }

    /// Structural change: persist and drop the cached canvas layout.
    fn after_mutation(&mut self) {
        self.layout = None;
        self.persist();
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let len = self.graph.nodes.len();
        let selected = (len > 0).then(|| self.list_state.selected().unwrap_or(0).min(len - 1));
        self.list_state.select(selected);
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.graph.nodes.len();
        if len == 0 {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        self.list_state
            .select(Some((current + delta).clamp(0, len as isize - 1) as usize));
        self.body_scroll = 0;
    }

    // --- Event handling -----------------------------------------------------

    fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.commit_editor();
            self.quit = true;
            return;
        }
        // Status messages are one-shot: any keypress dismisses them.
        self.status = None;
        match &mut self.mode {
            Mode::Browse => self.on_browse_key(key),
            Mode::TitleForm { .. } => self.on_title_key(key),
            Mode::LinkPicker { .. } => self.on_picker_key(key),
            Mode::ConfirmDelete { .. } => self.on_confirm_key(key),
            Mode::EditBody { .. } => self.on_editor_key(key),
        }
    }

    fn on_browse_key(&mut self, key: KeyEvent) {
        let tab_count = TAB_TITLES.len();
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            // Esc walks back to the graph tab first, then quits.
            KeyCode::Esc if self.tab == GRAPH_TAB => self.quit = true,
            KeyCode::Esc => self.tab = GRAPH_TAB,
            KeyCode::Tab => self.tab = (self.tab + 1) % tab_count,
            KeyCode::BackTab => self.tab = (self.tab + tab_count - 1) % tab_count,
            KeyCode::Char('1') => self.tab = GRAPH_TAB,
            KeyCode::Char('2') => self.tab = NODE_TAB,
            KeyCode::Char('3') => self.tab = VIZ_TAB,
            _ => match self.tab {
                GRAPH_TAB => self.on_graph_key(key),
                NODE_TAB => self.on_node_key(key),
                _ => self.on_viz_key(key),
            },
        }
    }

    fn on_graph_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('a') => {
                self.mode = Mode::TitleForm {
                    editing: None,
                    value: String::new(),
                    cursor: 0,
                };
            }
            KeyCode::Char('r') => {
                if let Some(index) = self.selected_index() {
                    let node = &self.graph.nodes[index];
                    self.mode = Mode::TitleForm {
                        editing: Some(node.id),
                        cursor: node.title.chars().count(),
                        value: node.title.clone(),
                    };
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self.selected_id() {
                    self.mode = Mode::ConfirmDelete { id };
                }
            }
            KeyCode::Char('l') => self.open_link_picker(),
            KeyCode::Enter if self.selected_id().is_some() => self.tab = NODE_TAB,
            KeyCode::Char('e') if self.selected_id().is_some() => {
                self.start_editor();
                self.tab = NODE_TAB;
            }
            _ => {}
        }
    }

    fn on_node_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.body_scroll = self.body_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.body_scroll = self.body_scroll.saturating_add(1);
            }
            KeyCode::Char('e') | KeyCode::Char('i') | KeyCode::Enter => {
                self.start_editor();
            }
            KeyCode::Char('l') => self.open_link_picker(),
            _ => {}
        }
    }

    fn on_viz_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Left => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Right => self.move_selection(1),
            KeyCode::Char('r') => {
                self.layout_seed = self.layout_seed.wrapping_add(1);
                self.layout = None;
            }
            KeyCode::Enter if self.selected_id().is_some() => self.tab = NODE_TAB,
            _ => {}
        }
    }

    fn open_link_picker(&mut self) {
        let Some(id) = self.selected_id() else {
            return;
        };
        let others: Vec<u64> = self
            .graph
            .nodes
            .iter()
            .map(|node| node.id)
            .filter(|other| *other != id)
            .collect();
        if others.is_empty() {
            self.status = Some(("Add another node before linking.".to_string(), true));
            return;
        }
        let mut state = ListState::default();
        state.select(Some(0));
        self.mode = Mode::LinkPicker { others, state };
    }

    /// Open the markdown editor for the selected node; no-op if none selected.
    fn start_editor(&mut self) {
        let Some(index) = self.selected_index() else {
            return;
        };
        self.mode = Mode::EditBody {
            editor: Editor::from_text(&self.graph.nodes[index].markdown),
        };
    }

    /// Write the open editor (if any) back to the node and persist.
    fn commit_editor(&mut self) {
        if let Mode::EditBody { editor } = &self.mode {
            let text = editor.text();
            if let Some(index) = self.selected_index() {
                self.graph.nodes[index].markdown = text;
            }
            self.persist();
        }
    }

    fn on_title_key(&mut self, key: KeyEvent) {
        let Mode::TitleForm {
            editing,
            value,
            cursor,
        } = &mut self.mode
        else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::Enter => {
                let title = value.trim().to_string();
                if title.is_empty() {
                    self.status = Some(("Title is required.".to_string(), true));
                    return;
                }
                match *editing {
                    Some(id) => {
                        if let Some(node) = self.graph.node_mut(id) {
                            node.title = title;
                        }
                        self.status = Some(("Renamed node.".to_string(), false));
                    }
                    None => {
                        self.graph.add_node(title);
                        self.list_state.select(Some(self.graph.nodes.len() - 1));
                        self.status = Some(("Added node.".to_string(), false));
                    }
                }
                self.mode = Mode::Browse;
                self.after_mutation();
            }
            KeyCode::Left => *cursor = cursor.saturating_sub(1),
            KeyCode::Right => *cursor = (*cursor + 1).min(value.chars().count()),
            KeyCode::Home => *cursor = 0,
            KeyCode::End => *cursor = value.chars().count(),
            KeyCode::Backspace if *cursor > 0 => {
                *cursor -= 1;
                let at = byte_index(value, *cursor);
                value.remove(at);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let at = byte_index(value, *cursor);
                value.insert(at, c);
                *cursor += 1;
            }
            _ => {}
        }
    }

    fn on_picker_key(&mut self, key: KeyEvent) {
        if self.selected_id().is_none() {
            self.mode = Mode::Browse;
            return;
        }
        let Mode::LinkPicker { others, state } = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => self.mode = Mode::Browse,
            KeyCode::Up | KeyCode::Char('k') => {
                let current = state.selected().unwrap_or(0) as isize;
                state.select(Some((current - 1).max(0) as usize));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let current = state.selected().unwrap_or(0) as isize;
                state.select(Some((current + 1).min(others.len() as isize - 1) as usize));
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_picker_link(),
            _ => {}
        }
    }

    /// Toggle the link between the selected node and the picker's highlighted
    /// row (shared by the picker's enter/space keys and mouse clicks).
    fn toggle_picker_link(&mut self) {
        let Some(selected_id) = self.selected_id() else {
            return;
        };
        let Mode::LinkPicker { others, state } = &self.mode else {
            return;
        };
        let Some(other) = state
            .selected()
            .and_then(|index| others.get(index).copied())
        else {
            return;
        };
        let linked = self.graph.toggle_edge(selected_id, other);
        self.after_mutation();
        let verb = if linked { "Linked" } else { "Unlinked" };
        self.status = Some((format!("{verb}."), false));
    }

    // --- Mouse handling -----------------------------------------------------

    fn on_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Like keypresses, a click dismisses any one-shot status.
                self.status = None;
                self.on_click(mouse.column, mouse.row);
            }
            MouseEventKind::ScrollUp => self.on_scroll(-1),
            MouseEventKind::ScrollDown => self.on_scroll(1),
            _ => {}
        }
    }

    fn on_click(&mut self, col: u16, row: u16) {
        match self.mode {
            Mode::Browse => {}
            Mode::LinkPicker { .. } => return self.on_picker_click(col, row),
            // Text-entry and confirm dialogs keep keyboard focus.
            _ => return,
        }
        if let Some((tabs_y, ranges)) = &self.hit.tabs {
            if row == *tabs_y {
                if let Some(tab) = ranges
                    .iter()
                    .position(|(start, end)| col >= *start && col < *end)
                {
                    self.tab = tab;
                }
                return;
            }
        }
        match self.tab {
            GRAPH_TAB => {
                let Some(rect) = self.hit.node_list else {
                    return;
                };
                if !rect.contains(Position::new(col, row)) {
                    return;
                }
                let index = self.list_state.offset() + (row - rect.y) as usize;
                if index < self.graph.nodes.len() {
                    self.list_state.select(Some(index));
                    self.body_scroll = 0;
                }
            }
            VIZ_TAB => {
                if let Some(index) = self.canvas_hit(col, row) {
                    self.list_state.select(Some(index));
                    self.body_scroll = 0;
                }
            }
            _ => {}
        }
    }

    fn on_picker_click(&mut self, col: u16, row: u16) {
        let Some(rect) = self.hit.picker else {
            return;
        };
        // Clicking outside the popup dismisses it, like esc.
        if !rect.contains(Position::new(col, row)) {
            self.mode = Mode::Browse;
            return;
        }
        let Mode::LinkPicker { others, state } = &mut self.mode else {
            return;
        };
        let index = state.offset() + (row - rect.y) as usize;
        if index < others.len() {
            state.select(Some(index));
            self.toggle_picker_link();
        }
    }

    /// Find the node whose dot or label sits at the clicked canvas cell.
    /// Vertical distance is weighted double because terminal cells are roughly
    /// twice as tall as they are wide.
    fn canvas_hit(&self, col: u16, row: u16) -> Option<usize> {
        let (area, x_bounds, y_bounds) = self.hit.canvas?;
        if !area.contains(Position::new(col, row)) {
            return None;
        }
        let positions = self.layout.as_ref()?;
        let span_x = x_bounds[1] - x_bounds[0];
        let span_y = y_bounds[1] - y_bounds[0];
        let mut best: Option<(f64, usize)> = None;
        for (index, (x, y)) in positions.iter().enumerate() {
            let cell_x = area.x as f64 + (x - x_bounds[0]) / span_x * area.width as f64;
            // Canvas y grows upward; cell rows grow downward.
            let cell_y = area.y as f64 + (y_bounds[1] - y) / span_y * area.height as f64;
            let dx = col as f64 + 0.5 - cell_x;
            let dy = row as f64 + 0.5 - cell_y;
            // A click anywhere on the "● label" text counts as a direct hit.
            let label_len = truncate(&self.graph.nodes[index].title, VIZ_LABEL_CHARS)
                .chars()
                .count() as f64
                + 2.0;
            let dx = if dy.abs() <= 1.0 && (-1.0..=label_len).contains(&dx) {
                0.0
            } else {
                dx
            };
            let score = dx * dx + (2.0 * dy) * (2.0 * dy);
            if best.is_none_or(|(best_score, _)| score < best_score) {
                best = Some((score, index));
            }
        }
        // Ignore clicks on empty space far from every node.
        best.filter(|(score, _)| *score <= 20.0)
            .map(|(_, index)| index)
    }

    fn on_scroll(&mut self, delta: isize) {
        match &mut self.mode {
            Mode::Browse => {}
            Mode::LinkPicker { others, state } => {
                let current = state.selected().unwrap_or(0) as isize;
                state.select(Some(
                    (current + delta).clamp(0, others.len() as isize - 1) as usize
                ));
                return;
            }
            _ => return,
        }
        match self.tab {
            NODE_TAB => {
                self.body_scroll = if delta < 0 {
                    self.body_scroll.saturating_sub(1)
                } else {
                    self.body_scroll.saturating_add(1)
                };
            }
            _ => self.move_selection(delta),
        }
    }

    fn on_confirm_key(&mut self, key: KeyEvent) {
        let Mode::ConfirmDelete { id } = &self.mode else {
            return;
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.graph.remove_node(*id);
                self.mode = Mode::Browse;
                self.after_mutation();
                self.status = Some(("Deleted node.".to_string(), false));
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.mode = Mode::Browse,
            _ => {}
        }
    }

    fn on_editor_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.commit_editor();
            self.status = Some(("Saved note.".to_string(), false));
            return;
        }
        if key.code == KeyCode::Esc {
            self.commit_editor();
            self.mode = Mode::Browse;
            self.status = Some(("Saved note.".to_string(), false));
            return;
        }
        let Mode::EditBody { editor } = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Enter => editor.insert_newline(),
            KeyCode::Backspace => editor.backspace(),
            KeyCode::Delete => editor.delete(),
            KeyCode::Up => editor.move_vertical(-1),
            KeyCode::Down => editor.move_vertical(1),
            KeyCode::Left => {
                if editor.col > 0 {
                    editor.col -= 1;
                } else if editor.row > 0 {
                    editor.row -= 1;
                    editor.col = editor.line_len();
                }
            }
            KeyCode::Right => {
                if editor.col < editor.line_len() {
                    editor.col += 1;
                } else if editor.row + 1 < editor.lines.len() {
                    editor.row += 1;
                    editor.col = 0;
                }
            }
            KeyCode::Home => editor.col = 0,
            KeyCode::End => editor.col = editor.line_len(),
            KeyCode::PageUp => editor.move_vertical(-10),
            KeyCode::PageDown => editor.move_vertical(10),
            KeyCode::Tab => {
                editor.insert_char(' ');
                editor.insert_char(' ');
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                editor.insert_char(c);
            }
            _ => {}
        }
    }

    // --- Rendering ----------------------------------------------------------

    fn draw(&mut self, frame: &mut Frame) {
        let [tabs_area, body_area, status_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        // Rebuilt every frame so mouse hit-testing always matches the screen.
        self.hit = HitAreas::default();
        self.hit.tabs = Some((tabs_area.y, tab_ranges(tabs_area.x)));

        let tabs = Tabs::new(TAB_TITLES)
            .select(self.tab)
            .highlight_style(Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, tabs_area);

        match self.tab {
            GRAPH_TAB => self.draw_graph_tab(frame, body_area),
            NODE_TAB => self.draw_node_tab(frame, body_area),
            _ => self.draw_viz_tab(frame, body_area),
        }

        self.draw_status(frame, status_area);
        frame.render_widget(
            Paragraph::new(self.help_text()).style(Style::new().fg(Color::DarkGray)),
            help_area,
        );

        match &self.mode {
            Mode::TitleForm { .. } => self.draw_title_form(frame),
            Mode::LinkPicker { .. } => self.draw_link_picker(frame),
            Mode::ConfirmDelete { .. } => draw_confirm(frame),
            Mode::Browse | Mode::EditBody { .. } => {}
        }
    }

    fn draw_graph_tab(&mut self, frame: &mut Frame, area: Rect) {
        if self.graph.nodes.is_empty() {
            frame.render_widget(
                Paragraph::new("No nodes yet — press a to add one.")
                    .style(Style::new().fg(Color::DarkGray)),
                area,
            );
            return;
        }
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Fill(2), Constraint::Fill(3)]).areas(area);

        let items: Vec<ListItem> = self
            .graph
            .nodes
            .iter()
            .map(|node| {
                let degree = self.graph.degree(node.id);
                ListItem::new(Line::from(vec![
                    Span::raw(node.title.clone()),
                    Span::styled(format!("  ({degree})"), Style::new().fg(Color::DarkGray)),
                ]))
            })
            .collect();
        let block = Block::bordered().title(" Nodes ");
        self.hit.node_list = Some(block.inner(list_area));
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, list_area, &mut self.list_state);

        let block = Block::bordered().title(" Details ");
        let inner = block.inner(detail_area);
        frame.render_widget(block, detail_area);
        let Some(index) = self.selected_index() else {
            return;
        };
        let node = &self.graph.nodes[index];
        let neighbor_titles: Vec<String> = self
            .graph
            .neighbors(node.id)
            .into_iter()
            .filter_map(|id| {
                self.graph
                    .nodes
                    .iter()
                    .find(|other| other.id == id)
                    .map(|other| other.title.clone())
            })
            .collect();
        let linked = if neighbor_titles.is_empty() {
            Line::styled(
                "Not linked to anything — press l to link.",
                Style::new().fg(Color::DarkGray),
            )
        } else {
            Line::from(vec![
                Span::styled("Linked: ", Style::new().fg(Color::DarkGray)),
                Span::styled(neighbor_titles.join(", "), Style::new().fg(Color::Cyan)),
            ])
        };
        let mut lines = vec![
            Line::styled(
                node.title.clone(),
                Style::new().add_modifier(Modifier::BOLD),
            ),
            linked,
            Line::raw(""),
        ];
        if node.markdown.trim().is_empty() {
            lines.push(Line::styled(
                "No notes yet — press e to edit.",
                Style::new().fg(Color::DarkGray),
            ));
        } else {
            lines.extend(styled_markdown(&node.markdown));
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
            inner,
        );
    }

    fn draw_node_tab(&mut self, frame: &mut Frame, area: Rect) {
        let Some(index) = self.selected_index() else {
            frame.render_widget(
                Paragraph::new("No node selected — add or select one on the Graph tab.")
                    .style(Style::new().fg(Color::DarkGray)),
                area,
            );
            return;
        };
        let [title_area, body_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        let node = &self.graph.nodes[index];
        let editing = matches!(self.mode, Mode::EditBody { .. });
        let suffix = if editing { " — editing" } else { "" };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    node.title.clone(),
                    Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(suffix, Style::new().fg(Color::Yellow)),
            ])),
            title_area,
        );

        if let Mode::EditBody { editor } = &mut self.mode {
            draw_editor(frame, body_area, editor);
        } else if node.markdown.trim().is_empty() {
            frame.render_widget(
                Paragraph::new("No notes yet — press e to edit.")
                    .style(Style::new().fg(Color::DarkGray)),
                body_area,
            );
        } else {
            let lines = styled_markdown(&node.markdown);
            self.body_scroll = self.body_scroll.min(lines.len().saturating_sub(1) as u16);
            frame.render_widget(
                Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .scroll((self.body_scroll, 0)),
                body_area,
            );
        }
    }

    fn draw_viz_tab(&mut self, frame: &mut Frame, area: Rect) {
        if self.graph.nodes.is_empty() {
            frame.render_widget(
                Paragraph::new("Nothing to visualize yet — add nodes on the Graph tab.")
                    .style(Style::new().fg(Color::DarkGray)),
                area,
            );
            return;
        }
        if self.layout.is_none() {
            let index_of: HashMap<u64, usize> = self
                .graph
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.id, index))
                .collect();
            let edges: Vec<(usize, usize)> = self
                .graph
                .edges
                .iter()
                .filter_map(|(a, b)| Some((*index_of.get(a)?, *index_of.get(b)?)))
                .collect();
            self.layout = Some(force_layout(
                self.graph.nodes.len(),
                &edges,
                self.layout_seed,
            ));
        }
        let positions = self.layout.clone().expect("computed above");
        let selected = self.selected_index();

        // Bounding box with padding; extra room on the right so labels printed
        // beside their nodes are less likely to clip at the edge.
        let min_x = positions.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = positions
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = positions.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = positions
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let span_x = (max_x - min_x).max(0.5);
        let span_y = (max_y - min_y).max(0.5);
        let x_bounds = [min_x - 0.12 * span_x, max_x + 0.35 * span_x];
        let y_bounds = [min_y - 0.15 * span_y, max_y + 0.15 * span_y];
        self.hit.canvas = Some((area, x_bounds, y_bounds));

        let index_of: HashMap<u64, usize> = self
            .graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, index))
            .collect();
        let graph = &self.graph;
        let canvas = Canvas::default()
            .marker(Marker::Braille)
            .x_bounds(x_bounds)
            .y_bounds(y_bounds)
            .paint(move |ctx| {
                for (a, b) in &graph.edges {
                    let (Some(&ai), Some(&bi)) = (index_of.get(a), index_of.get(b)) else {
                        continue;
                    };
                    let touches_selected = selected == Some(ai) || selected == Some(bi);
                    ctx.draw(&CanvasLine {
                        x1: positions[ai].0,
                        y1: positions[ai].1,
                        x2: positions[bi].0,
                        y2: positions[bi].1,
                        color: if touches_selected {
                            Color::Cyan
                        } else {
                            Color::DarkGray
                        },
                    });
                }
                // Labels paint over the braille layer, so they stay readable
                // even where edges cross.
                for (index, (x, y)) in positions.iter().enumerate() {
                    let is_selected = selected == Some(index);
                    let dot_style = if is_selected {
                        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(Color::Blue)
                    };
                    let label_style = if is_selected {
                        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(Color::Gray)
                    };
                    let title = truncate(&graph.nodes[index].title, VIZ_LABEL_CHARS);
                    ctx.print(
                        *x,
                        *y,
                        Line::from(vec![
                            Span::styled("●", dot_style),
                            Span::styled(format!(" {title}"), label_style),
                        ]),
                    );
                }
            });
        frame.render_widget(canvas, area);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let (text, style) = match &self.status {
            Some((message, true)) => (message.clone(), Style::new().fg(Color::Red)),
            Some((message, false)) => (message.clone(), Style::new().fg(Color::Green)),
            None => {
                let mut text = format!(
                    "{} node(s) · {} link(s)",
                    self.graph.nodes.len(),
                    self.graph.edges.len()
                );
                if let Some(id) = self.selected_id() {
                    text.push_str(&format!(
                        " · selected has {} link(s)",
                        self.graph.degree(id)
                    ));
                }
                (text, Style::new().fg(Color::DarkGray))
            }
        };
        frame.render_widget(Paragraph::new(text).style(style), area);
    }

    fn draw_title_form(&self, frame: &mut Frame) {
        let Mode::TitleForm {
            editing,
            value,
            cursor,
        } = &self.mode
        else {
            return;
        };
        let area = centered(frame.area(), 50, 3);
        frame.render_widget(Clear, area);
        let title = if editing.is_some() {
            " Rename node "
        } else {
            " Add node "
        };
        let block = Block::bordered().title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        // Keep the cursor visible when the title is wider than the field.
        let available = inner.width as usize;
        let skip = cursor.saturating_sub(available.saturating_sub(1));
        let visible: String = value.chars().skip(skip).take(available).collect();
        frame.render_widget(Paragraph::new(visible), inner);
        frame.set_cursor_position(Position::new(inner.x + (cursor - skip) as u16, inner.y));
    }

    fn draw_link_picker(&mut self, frame: &mut Frame) {
        let Some(selected_id) = self.selected_id() else {
            return;
        };
        let Mode::LinkPicker { others, state } = &mut self.mode else {
            return;
        };
        let height = (others.len() as u16 + 2)
            .min(frame.area().height.saturating_sub(2))
            .max(3);
        let area = centered(frame.area(), 46, height);
        frame.render_widget(Clear, area);
        let block = Block::bordered().title(" Link / unlink ");
        let inner = block.inner(area);
        self.hit.picker = Some(inner);
        frame.render_widget(block, area);
        let items: Vec<ListItem> = others
            .iter()
            .map(|other| {
                let linked = self.graph.is_linked(selected_id, *other);
                let title = self
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.id == *other)
                    .map(|node| node.title.as_str())
                    .unwrap_or("?");
                let (mark, style) = if linked {
                    ("✓ ", Style::new().fg(Color::Cyan))
                } else {
                    ("  ", Style::new())
                };
                ListItem::new(Line::from(vec![
                    Span::styled(mark, Style::new().fg(Color::Cyan)),
                    Span::styled(title.to_string(), style),
                ]))
            })
            .collect();
        let list = List::new(items).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, inner, state);
    }

    fn help_text(&self) -> &'static str {
        match (&self.mode, self.tab) {
            (Mode::TitleForm { .. }, _) => "enter save · esc cancel",
            (Mode::LinkPicker { .. }, _) => "↑/↓ select · enter/space toggle link · esc close",
            (Mode::ConfirmDelete { .. }, _) => "y delete · n/esc cancel",
            (Mode::EditBody { .. }, _) => "esc save & exit · ctrl+s save · arrows/home/end move",
            (Mode::Browse, GRAPH_TAB) => {
                "↑/↓ select · a add · r rename · l link · d delete · enter view · e edit note · tab switch · q quit"
            }
            (Mode::Browse, NODE_TAB) => "↑/↓ scroll · e edit · l link · tab switch · esc back · q quit",
            (Mode::Browse, _) => "↑/↓ cycle node · r shuffle layout · enter view · tab switch · esc back · q quit",
        }
    }
}

fn draw_editor(frame: &mut Frame, area: Rect, editor: &mut Editor) {
    let height = area.height as usize;
    let width = area.width as usize;
    if height == 0 || width == 0 {
        return;
    }
    // Keep the cursor's row inside the visible window.
    if editor.row < editor.scroll {
        editor.scroll = editor.row;
    } else if editor.row >= editor.scroll + height {
        editor.scroll = editor.row + 1 - height;
    }
    // Keep the cursor's column visible on its own line.
    let hskip = editor.col.saturating_sub(width.saturating_sub(1));
    let lines: Vec<Line> = editor
        .lines
        .iter()
        .enumerate()
        .skip(editor.scroll)
        .take(height)
        .map(|(row, line)| {
            let skip = if row == editor.row { hskip } else { 0 };
            Line::raw(line.chars().skip(skip).take(width).collect::<String>())
        })
        .collect();
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
    frame.set_cursor_position(Position::new(
        area.x + (editor.col - hskip) as u16,
        area.y + (editor.row - editor.scroll) as u16,
    ));
}

fn draw_confirm(frame: &mut Frame) {
    let message = "Delete this node and its links? (y/n)";
    let area = centered(frame.area(), message.chars().count() as u16 + 4, 3);
    frame.render_widget(Clear, area);
    let block = Block::bordered().title(" Confirm ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(message).centered(), inner);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Style one markdown document as terminal lines: headings colored by level,
/// list bullets and blockquotes accented, fenced code blocks dimmed. This is
/// intentionally line-based — enough to make notes scannable, not a full
/// renderer.
fn styled_markdown(text: &str) -> Vec<Line<'static>> {
    let mut in_fence = false;
    text.lines()
        .map(|raw| {
            let trimmed = raw.trim_start();
            if trimmed.starts_with("```") {
                in_fence = !in_fence;
                return Line::styled(raw.to_string(), Style::new().fg(Color::DarkGray));
            }
            if in_fence {
                return Line::styled(raw.to_string(), Style::new().fg(Color::DarkGray));
            }
            if raw.starts_with('#') {
                let level = raw.chars().take_while(|c| *c == '#').count();
                let color = match level {
                    1 => Color::Magenta,
                    2 => Color::Blue,
                    _ => Color::Cyan,
                };
                return Line::styled(
                    raw.to_string(),
                    Style::new().fg(color).add_modifier(Modifier::BOLD),
                );
            }
            let indent = " ".repeat(raw.len() - trimmed.len());
            if let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("+ "))
            {
                return Line::from(vec![
                    Span::raw(indent),
                    Span::styled("• ", Style::new().fg(Color::Yellow)),
                    Span::raw(rest.to_string()),
                ]);
            }
            if let Some((num, rest)) = trimmed.split_once(' ') {
                if num.len() >= 2
                    && num.ends_with('.')
                    && num[..num.len() - 1].chars().all(|c| c.is_ascii_digit())
                {
                    return Line::from(vec![
                        Span::raw(indent),
                        Span::styled(format!("{num} "), Style::new().fg(Color::Yellow)),
                        Span::raw(rest.to_string()),
                    ]);
                }
            }
            if trimmed.starts_with('>') {
                return Line::styled(
                    raw.to_string(),
                    Style::new().fg(Color::Green).add_modifier(Modifier::ITALIC),
                );
            }
            Line::raw(raw.to_string())
        })
        .collect()
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
fn force_layout(n: usize, edges: &[(usize, usize)], seed: u64) -> Vec<(f64, f64)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_edits_roundtrip() {
        let mut editor = Editor::from_text("hello\nworld");
        assert_eq!(editor.lines.len(), 2);
        editor.col = editor.line_len(); // end of "hello"
        editor.insert_char('!');
        editor.insert_newline();
        editor.insert_char('x');
        assert_eq!(editor.text(), "hello!\nx\nworld");
        editor.backspace();
        editor.backspace(); // joins back onto "hello!"
        assert_eq!(editor.text(), "hello!\nworld");
        assert_eq!((editor.row, editor.col), (0, 6));
        editor.delete(); // joins "world" up
        assert_eq!(editor.text(), "hello!world");
    }

    #[test]
    fn editor_handles_multibyte_chars() {
        let mut editor = Editor::from_text("");
        for c in "héllo".chars() {
            editor.insert_char(c);
        }
        assert_eq!(editor.text(), "héllo");
        editor.col = 2; // between é and l
        editor.backspace();
        assert_eq!(editor.text(), "hllo");
    }

    #[test]
    fn force_layout_is_deterministic_and_finite() {
        let edges = [(0, 1), (1, 2), (2, 0), (2, 3)];
        let a = force_layout(5, &edges, 42);
        let b = force_layout(5, &edges, 42);
        assert_eq!(a, b);
        assert_eq!(a.len(), 5);
        assert!(a.iter().all(|(x, y)| x.is_finite() && y.is_finite()));
        // All nodes end up distinct (repulsion worked).
        for i in 0..a.len() {
            for j in (i + 1)..a.len() {
                let (dx, dy) = (a[i].0 - a[j].0, a[i].1 - a[j].1);
                assert!(dx * dx + dy * dy > 1e-4);
            }
        }
    }

    #[test]
    fn force_layout_pulls_linked_nodes_closer() {
        // A path 0-1 plus an isolated node 2: the linked pair should sit
        // closer together than either sits to the loner.
        let positions = force_layout(3, &[(0, 1)], 7);
        let dist = |i: usize, j: usize| {
            let (dx, dy) = (
                positions[i].0 - positions[j].0,
                positions[i].1 - positions[j].1,
            );
            (dx * dx + dy * dy).sqrt()
        };
        assert!(dist(0, 1) < dist(0, 2));
        assert!(dist(0, 1) < dist(1, 2));
    }

    #[test]
    fn tab_ranges_match_tabs_widget_layout() {
        // The widget renders " Graph │ Node │ Visualize": one padding cell
        // each side of a title, one divider cell between tabs.
        assert_eq!(tab_ranges(0), vec![(0, 7), (8, 14), (15, 26)]);
        // Ranges shift with the row's origin.
        assert_eq!(tab_ranges(2)[0], (2, 9));
    }

    #[test]
    fn canvas_click_selects_nearest_node() {
        let mut graph = Graph::default();
        graph.add_node("A".to_string());
        graph.add_node("B".to_string());
        let mut app = App::new(graph, PathBuf::from("/dev/null"));
        app.hit.canvas = Some((Rect::new(0, 0, 100, 40), [0.0, 1.0], [0.0, 1.0]));
        // A at cell (25, 20), B at (75, 20) — canvas y grows upward.
        app.layout = Some(vec![(0.25, 0.5), (0.75, 0.5)]);
        assert_eq!(app.canvas_hit(25, 20), Some(0));
        // A click on B's label (a few cells right of its dot) still hits B.
        assert_eq!(app.canvas_hit(77, 20), Some(1));
        // Clicks on empty space and outside the canvas hit nothing.
        assert_eq!(app.canvas_hit(50, 5), None);
        assert_eq!(app.canvas_hit(105, 20), None);
    }

    #[test]
    fn styled_markdown_highlights_structure() {
        let lines = styled_markdown("# Title\n- item\n```\ncode # not a heading\n```\nplain");
        assert_eq!(lines.len(), 6);
        // Heading is bold; the fence body is dimmed even though it contains '#'.
        assert!(lines[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(lines[3].style.fg, Some(Color::DarkGray));
        assert_eq!(lines[5].style.fg, None);
        // The bullet marker is replaced with a styled dot.
        assert_eq!(lines[1].spans[1].content, "• ");
    }
}
