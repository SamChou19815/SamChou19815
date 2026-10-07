//! Desktop app for `sam knowledge-graph` — straight Elm architecture on
//! [iced]: `update` is the only place state changes, `view` is a pure
//! function from [`State`] to an [`Element`] tree, and every interaction is
//! a [`Message`]. Raw keyboard input becomes messages through
//! [`subscription`]. The look is a warm gruvbox editor theme ([`theme`]).

mod canvas;
mod theme;
mod view;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use iced::keyboard::key::Named;
use iced::{event, keyboard, Point, Subscription, Task, Vector};

use super::knowledge_graph::{force_layout, save, Graph};

const TABS: [&str; 2] = ["Graph", "Visualize"];
const GRAPH_TAB: usize = 0;
const VIZ_TAB: usize = 1;

/// The text field of the title dialog, focused whenever it opens.
fn title_input() -> iced::widget::Id {
    iced::widget::Id::new("kg-title-form")
}

/// Every interaction in the app — the single input to [`update`].
#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(usize),
    CycleTab { back: bool },
    SelectNode(u64),
    OpenNote(u64),
    MoveSelection(isize),

    // sidebar
    SearchChanged(String),
    AddNode,

    // note actions
    Rename(u64),
    AskDelete(u64),
    LinkClicked,
    StartEditing,
    NoteEdited(iced::widget::text_editor::Action),
    NoteSaved,
    NoteClosed,

    // dialogs
    TitleChanged(String),
    TitleSubmitted,
    DialogCancelled,
    ToggleLink(u64),
    PickerMoved(isize),
    PickerToggled,
    ConfirmDelete,

    // canvas
    Pan(Vector),
    Zoom { factor: f32, at: Option<Point> },
    ShuffleLayout,
    FitView,

    // keyboard subscriptions
    KeyPressed(keyboard::Key, keyboard::Modifiers),
}

/// Modal dialogs.
enum Dialog {
    None,
    /// Add (`editing: None`) or rename (`Some(id)`) a note.
    TitleForm {
        editing: Option<u64>,
        value: String,
    },
    /// Pick another note and toggle the link to it. Stays open so several
    /// links can be toggled in one visit.
    LinkPicker {
        highlight: usize,
    },
    ConfirmDelete {
        id: u64,
    },
}

/// The application state. Everything the view reads, nothing it changes.
pub struct State {
    graph: Graph,
    path: PathBuf,
    tab: usize,
    /// Selected node id; stable across list edits, unlike an index.
    selected: Option<u64>,
    /// Id of the note the editor is bound to, if editing.
    editing: Option<u64>,
    /// The editor's live content, committed on every keystroke.
    draft: iced::widget::text_editor::Content,
    /// Sidebar filter over note titles.
    filter: String,
    dialog: Dialog,
    /// The graph canvas and its cached geometry.
    canvas: canvas::GraphCanvas,
    /// Re-seeded by `r` (or the shuffle control) to shake out a new layout.
    layout_seed: u64,
    /// One-shot message shown in the status line: `(text, is_error)`.
    status: Option<(String, bool)>,
}

// --- update -----------------------------------------------------------------

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::TabSelected(tab) => {
            state.tab = tab.min(TABS.len() - 1);
            state.editing = None;
        }
        Message::CycleTab { back } => {
            state.tab = 1 - state.tab;
            state.editing = None;
            let _ = back;
        }
        Message::SelectNode(id) => state.selected = Some(id),
        Message::OpenNote(id) => {
            state.selected = Some(id);
            state.tab = GRAPH_TAB;
        }
        Message::MoveSelection(delta) => {
            let len = state.graph.nodes.len();
            if len > 0 {
                let current = state.selected_index().unwrap_or(0) as isize;
                let index = (current + delta).clamp(0, len as isize - 1) as usize;
                state.selected = Some(state.graph.nodes[index].id);
            }
        }

        Message::SearchChanged(filter) => state.filter = filter,
        Message::AddNode => {
            open_title_form(state, None);
            return iced::widget::operation::focus(title_input());
        }

        Message::Rename(id) => {
            open_title_form(state, Some(id));
            return iced::widget::operation::focus(title_input());
        }
        Message::AskDelete(id) => state.dialog = Dialog::ConfirmDelete { id },
        Message::LinkClicked => {
            if let Some(id) = state.selected {
                if !state.graph.nodes.iter().any(|node| node.id != id) {
                    state.status = Some(("Add another note before linking.".into(), true));
                } else {
                    state.dialog = Dialog::LinkPicker { highlight: 0 };
                }
            }
        }
        Message::StartEditing => {
            if let Some(node) = state.selected_node().cloned() {
                state.editing = Some(node.id);
                state.draft = iced::widget::text_editor::Content::new();
                state.draft.perform(iced::widget::text_editor::Action::Edit(
                    iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                        node.markdown.clone(),
                    )),
                ));
            }
        }
        Message::NoteEdited(action) => {
            state.draft.perform(action);
            commit_note(state);
        }
        Message::NoteSaved => {
            commit_note(state);
            state.status = Some(("Saved note.".into(), false));
        }
        Message::NoteClosed => {
            commit_note(state);
            state.editing = None;
            state.status = Some(("Saved note.".into(), false));
        }

        Message::TitleChanged(value) => {
            if let Dialog::TitleForm { value: slot, .. } = &mut state.dialog {
                *slot = value;
            }
        }
        Message::TitleSubmitted => {
            let Dialog::TitleForm { editing, value } = &state.dialog else {
                return Task::none();
            };
            let title = value.trim().to_string();
            if title.is_empty() {
                state.status = Some(("Title is required.".into(), true));
                return Task::none();
            }
            match *editing {
                Some(id) => {
                    if let Some(node) = state.graph.node_mut(id) {
                        node.title = title;
                    }
                    state.status = Some(("Renamed note.".into(), false));
                }
                None => {
                    let id = state.graph.add_node(title);
                    state.selected = Some(id);
                    state.status = Some(("Added note.".into(), false));
                }
            }
            state.dialog = Dialog::None;
            after_mutation(state);
        }
        Message::DialogCancelled => state.dialog = Dialog::None,
        Message::ToggleLink(other) => {
            if let Some(id) = state.selected {
                let linked = state.graph.toggle_edge(id, other);
                after_mutation(state);
                let verb = if linked { "Linked." } else { "Unlinked." };
                state.status = Some((verb.into(), false));
            }
        }
        Message::PickerMoved(delta) => {
            if let Dialog::LinkPicker { highlight } = &mut state.dialog {
                let len = state.graph.nodes.len().saturating_sub(1);
                *highlight = (*highlight as isize + delta).clamp(0, len as isize) as usize;
            }
        }
        Message::PickerToggled => {
            if let Dialog::LinkPicker { highlight } = &state.dialog {
                if let Some(other) = state
                    .graph
                    .nodes
                    .iter()
                    .filter(|node| Some(node.id) != state.selected)
                    .nth(*highlight)
                {
                    let id = other.id;
                    return update(state, Message::ToggleLink(id));
                }
            }
        }
        Message::ConfirmDelete => {
            if let Dialog::ConfirmDelete { id } = state.dialog {
                state.graph.remove_node(id);
                state.dialog = Dialog::None;
                after_mutation(state);
                state.status = Some(("Deleted note.".into(), false));
            }
        }

        Message::Pan(delta) => state.canvas.pan(delta),
        Message::Zoom { factor, at } => state.canvas.zoom(factor, at),
        Message::ShuffleLayout => {
            state.layout_seed = state.layout_seed.wrapping_add(1);
            refresh_canvas(state);
        }
        Message::FitView => state.canvas.reset_view(),

        Message::KeyPressed(key, modifiers) => {
            let _ = on_key(state, key, modifiers);
        }
    }
    Task::none()
}

/// Global keyboard dispatch: the app shortcuts, expressed as messages.
fn on_key(state: &mut State, key: keyboard::Key, modifiers: keyboard::Modifiers) -> Task<Message> {
    use keyboard::Key;
    // Dialogs first.
    match &state.dialog {
        Dialog::LinkPicker { .. } => {
            return match key {
                Key::Named(Named::ArrowUp) => update(state, Message::PickerMoved(-1)),
                Key::Named(Named::ArrowDown) => update(state, Message::PickerMoved(1)),
                Key::Named(Named::Enter) | Key::Named(Named::Space) => {
                    update(state, Message::PickerToggled)
                }
                Key::Named(Named::Escape) => update(state, Message::DialogCancelled),
                _ => Task::none(),
            };
        }
        Dialog::ConfirmDelete { .. } => {
            return match key {
                Key::Named(Named::Enter) => update(state, Message::ConfirmDelete),
                Key::Character(ref c) if c.eq_ignore_ascii_case("y") => {
                    update(state, Message::ConfirmDelete)
                }
                Key::Named(Named::Escape) => update(state, Message::DialogCancelled),
                Key::Character(ref c) if c.eq_ignore_ascii_case("n") => {
                    update(state, Message::DialogCancelled)
                }
                _ => Task::none(),
            };
        }
        Dialog::TitleForm { .. } => {
            return match key {
                // Enter submits through the text input's on_submit; Escape
                // cancels.
                Key::Named(Named::Escape) => update(state, Message::DialogCancelled),
                _ => Task::none(),
            };
        }
        Dialog::None => {}
    }
    let has_selection = state.selected.is_some();
    let tab = state.tab;
    // While the note editor is open, Esc closes it and ctrl/cmd+s saves.
    if state.editing.is_some() {
        return match key {
            Key::Named(Named::Escape) => update(state, Message::NoteClosed),
            Key::Character(ref c) if c.eq_ignore_ascii_case("s") && modifiers.control() => {
                update(state, Message::NoteSaved)
            }
            _ => Task::none(),
        };
    }
    let browse = |state: &mut State| -> Task<Message> {
        match key {
            Key::Named(Named::Tab) => update(
                state,
                Message::CycleTab {
                    back: modifiers.shift(),
                },
            ),
            Key::Character(ref c) if c.eq_ignore_ascii_case("1") => {
                update(state, Message::TabSelected(GRAPH_TAB))
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("2") => {
                update(state, Message::TabSelected(VIZ_TAB))
            }
            Key::Named(Named::ArrowUp) => update(state, Message::MoveSelection(-1)),
            Key::Character(ref c) if c.eq_ignore_ascii_case("k") => {
                update(state, Message::MoveSelection(-1))
            }
            Key::Named(Named::ArrowDown) => update(state, Message::MoveSelection(1)),
            Key::Character(ref c) if c.eq_ignore_ascii_case("j") => {
                update(state, Message::MoveSelection(1))
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("a") && tab == GRAPH_TAB => {
                update(state, Message::AddNode)
            }
            Key::Character(ref c)
                if c.eq_ignore_ascii_case("r") && tab == GRAPH_TAB && has_selection =>
            {
                let id = state.selected.unwrap_or_default();
                update(state, Message::Rename(id))
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("d") && tab == GRAPH_TAB => {
                match state.selected {
                    Some(id) => update(state, Message::AskDelete(id)),
                    None => Task::none(),
                }
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("l") && tab == GRAPH_TAB => {
                update(state, Message::LinkClicked)
            }
            Key::Named(Named::Enter) if has_selection => update(state, Message::StartEditing),
            Key::Character(ref c)
                if (c.eq_ignore_ascii_case("e") || c.eq_ignore_ascii_case("i"))
                    && tab == GRAPH_TAB =>
            {
                update(state, Message::StartEditing)
            }
            Key::Character(ref c) if c.eq_ignore_ascii_case("r") && tab == VIZ_TAB => {
                update(state, Message::ShuffleLayout)
            }
            _ => Task::none(),
        }
    };
    browse(state)
}

// --- subscriptions ----------------------------------------------------------

/// Raw keyboard events become messages; keys captured by text inputs and the
/// editor are not forwarded, so shortcuts stay inert while typing.
fn subscription(_state: &State) -> Subscription<Message> {
    event::listen_with(|event, status, _window| match event {
        event::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            match status {
                event::Status::Ignored => Some(Message::KeyPressed(key.clone(), modifiers)),
                event::Status::Captured => None,
            }
        }
        _ => None,
    })
}

// --- model helpers ----------------------------------------------------------

impl State {
    fn selected_index(&self) -> Option<usize> {
        self.selected
            .and_then(|id| self.graph.nodes.iter().position(|node| node.id == id))
    }

    fn selected_node(&self) -> Option<&super::knowledge_graph::Node> {
        self.selected
            .and_then(|id| self.graph.nodes.iter().find(|node| node.id == id))
    }
}

fn open_title_form(state: &mut State, editing: Option<u64>) {
    let value = editing
        .and_then(|id| state.graph.node_mut(id))
        .map(|node| node.title.clone())
        .unwrap_or_default();
    state.dialog = Dialog::TitleForm { editing, value };
}

/// Write the editor draft (if open) back to its note and persist.
fn commit_note(state: &mut State) {
    let Some(id) = state.editing else {
        return;
    };
    if let Some(node) = state.graph.node_mut(id) {
        node.markdown = state.draft.text().to_string();
    }
    persist(state);
}

/// Persist the graph, reporting failure in the status line (the in-memory
/// state stays authoritative either way).
fn persist(state: &mut State) {
    if let Err(err) = save(&state.path, &state.graph) {
        state.status = Some((err.to_string(), true));
    }
}

/// Structural change: persist, recompute the canvas layout, and keep the
/// selection pointed at an existing note.
fn after_mutation(state: &mut State) {
    persist(state);
    refresh_canvas(state);
    let alive = |graph: &Graph, id: u64| graph.nodes.iter().any(|node| node.id == id);
    if state.selected.is_none_or(|id| !alive(&state.graph, id)) {
        state.selected = state.graph.nodes.first().map(|node| node.id);
    }
}

/// Recompute the force layout and push it into the canvas.
fn refresh_canvas(state: &mut State) {
    let n = state.graph.nodes.len();
    let index_of: HashMap<u64, usize> = state
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect();
    let edges: Vec<(usize, usize)> = state
        .graph
        .edges
        .iter()
        .filter_map(|(a, b)| Some((*index_of.get(a)?, *index_of.get(b)?)))
        .collect();
    let positions = force_layout(n, &edges, state.layout_seed);
    let selected = state.selected.and_then(|id| index_of.get(&id).copied());
    state.canvas.set_graph(
        state
            .graph
            .nodes
            .iter()
            .zip(&positions)
            .map(|(node, (x, y))| canvas::NodePoint {
                id: node.id,
                title: node.title.clone(),
                x: *x,
                y: *y,
            })
            .collect(),
        edges,
        selected,
    );
}

// --- entry point ------------------------------------------------------------

/// The initial graph handed over by `run`; `iced::application` takes plain
/// functions for boot/update/view, so the CLI-provided data waits here.
static BOOT: std::sync::Mutex<Option<(Graph, PathBuf)>> = std::sync::Mutex::new(None);

fn app_title(_state: &State) -> String {
    "sam knowledge-graph".to_string()
}

fn app_theme(_state: &State) -> iced::Theme {
    theme::theme()
}

fn boot() -> State {
    let (graph, path) = BOOT
        .lock()
        .expect("boot data present")
        .take()
        .expect("boot runs once");
    State::new(graph, path)
}

pub fn run(graph: Graph, path: PathBuf) -> Result<()> {
    *BOOT.lock().expect("boot data present") = Some((graph, path));
    iced::application(boot, update, view::view)
        .title(app_title)
        .theme(app_theme)
        .subscription(subscription)
        .window_size((1080.0, 720.0))
        .run()
        .map_err(|err| anyhow::anyhow!("opening the knowledge graph window: {err}"))
}

impl State {
    fn new(graph: Graph, path: PathBuf) -> Self {
        let mut state = State {
            selected: graph.nodes.first().map(|node| node.id),
            graph,
            path,
            tab: GRAPH_TAB,
            editing: None,
            draft: iced::widget::text_editor::Content::new(),
            filter: String::new(),
            dialog: Dialog::None,
            canvas: canvas::GraphCanvas::new(),
            layout_seed: 0x5EED,
            status: None,
        };
        refresh_canvas(&mut state);
        state
    }
}
