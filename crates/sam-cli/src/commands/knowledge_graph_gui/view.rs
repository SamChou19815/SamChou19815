//! The view: a pure function from [`State`] to an [`Element`] tree. Every
//! interaction is declared as the message it should produce — `on_press`,
//! `on_input`, `on_submit` — and nothing here mutates anything.

use iced::widget::scrollable::Scrollbar;
use iced::widget::{
    button, column, container, mouse_area, row, rule, scrollable, text, text_editor, text_input,
    Space,
};
use iced::{Alignment, Element, Length, Padding};

use super::theme;
use super::{Dialog, Message, State, GRAPH_TAB, TABS};

pub fn view(state: &State) -> Element<'_, Message> {
    let body = match state.tab {
        GRAPH_TAB => graph_tab(state),
        _ => viz_tab(state),
    };
    let app = column![
        column![
            top_bar(state),
            rule::horizontal(1.0).style(|_theme| rule_style()),
        ],
        body,
        status_bar(state),
    ]
    .width(Length::Fill)
    .height(Length::Fill);
    match dialog(state) {
        Some(dialog) => overlay(app.into(), dialog),
        None => app.into(),
    }
}

// --- text helpers -----------------------------------------------------------

fn label(content: impl Into<String>, size: f32, color: iced::Color) -> Element<'static, Message> {
    text(content.into()).size(size).color(color).into()
}

// --- chrome -----------------------------------------------------------------

fn top_bar(state: &State) -> Element<'_, Message> {
    let tabs: Vec<Element<'_, Message>> = TABS
        .iter()
        .enumerate()
        .map(|(index, label)| tab(label, index == state.tab, index))
        .collect();
    let tabs = row(tabs).spacing(18.0);
    container(tabs)
        .width(Length::Fill)
        .height(42)
        .center_x(Length::Fill)
        .align_y(Alignment::Center)
        .style(|_theme| theme::chrome())
        .into()
}

/// One tab: the active one in the accent color, the rest muted; hover wash
/// on both.
fn tab(label: &str, active: bool, index: usize) -> Element<'_, Message> {
    let color = if active { theme::ACCENT } else { theme::MUTED };
    button(text(label.to_string()).size(13.0).color(color))
        .on_press(Message::TabSelected(index))
        .padding([8.0, 12.0])
        .style(move |_theme, status| hover_wash(status))
        .into()
}

fn hover_wash(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(theme::HOVER.into()),
            border: iced::border::Border {
                radius: theme::rounded(6.0),
                ..Default::default()
            },
            ..Default::default()
        },
        _ => button::Style::default(),
    }
}

fn status_bar(state: &State) -> Element<'_, Message> {
    let (line, color) = match &state.status {
        Some((message, true)) => (message.clone(), theme::DANGER),
        Some((message, false)) => (message.clone(), theme::SUCCESS),
        None => {
            let mut text = format!(
                "{} notes · {} links",
                state.graph.nodes.len(),
                state.graph.edges.len()
            );
            if let Some(id) = state.selected {
                text.push_str(&format!(" · {} linked to this", state.graph.degree(id)));
            }
            (text, theme::MUTED)
        }
    };
    let left: Element<'_, Message> = text(line).size(11.5).color(color).into();
    let right: Element<'_, Message> = text(help(state).to_string())
        .size(11.5)
        .color(theme::MUTED)
        .into();
    let bar = row![left, Space::new().width(Length::Fill), right]
        .align_y(Alignment::Center)
        .width(Length::Fill);
    container(bar)
        .height(26)
        .width(Length::Fill)
        .padding(Padding::new(0.0).left(14).right(14))
        .style(|_theme| theme::chrome())
        .align_y(Alignment::Center)
        .into()
}

fn help(state: &State) -> &'static str {
    match (&state.dialog, state.editing, state.tab) {
        (Dialog::TitleForm { .. }, _, _) => "enter save · esc cancel",
        (Dialog::LinkPicker { .. }, _, _) => "↑/↓ select · enter/space toggle · esc close",
        (Dialog::ConfirmDelete { .. }, _, _) => "y delete · n/esc cancel",
        (_, Some(_), _) => "esc save & exit · ctrl+s save",
        (_, None, GRAPH_TAB) => {
            "↑/↓ select · a add · r rename · l link · d delete · enter edit · tab switch"
        }
        (_, None, _) => "↑/↓ cycle · r shuffle · enter edit · drag pan · scroll zoom · tab switch",
    }
}

// --- graph tab --------------------------------------------------------------

fn graph_tab(state: &State) -> Element<'_, Message> {
    let divider = container(Space::new().width(1.0).height(Length::Fill))
        .style(|_theme| theme::vertical_rule());
    let main = container(details(state))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20);
    row![sidebar(state), divider, main]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// The sidebar: search over Bear-style note rows (title + first-line
/// snippet).
fn sidebar(state: &State) -> Element<'_, Message> {
    let needle = state.filter.to_lowercase();
    let rows: Vec<Element<'_, Message>> = state
        .graph
        .nodes
        .iter()
        .filter(|node| needle.is_empty() || node.title.to_lowercase().contains(&needle))
        .map(|node| note_row(node, state))
        .collect();
    let list: Element<'_, Message> = if rows.is_empty() {
        label("No notes yet — press a to add one.", 11.5, theme::MUTED)
    } else {
        column(rows).spacing(2.0).into()
    };
    let add: Element<'_, Message> = button(text("+ New note").size(12.5).color(theme::MUTED))
        .on_press(Message::AddNode)
        .style(move |_theme, status| hover_wash(status))
        .into();
    let header = row![
        label(
            format!("Notes · {}", state.graph.nodes.len()),
            11.5,
            theme::MUTED,
        ),
        Space::new().width(Length::Fill),
        add,
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);
    let search = text_input("Search", &state.filter)
        .on_input(Message::SearchChanged)
        .size(13.0)
        .style(|_theme, _status| theme::text_input());
    let pane = column![
        header,
        search,
        scrollable(column![list].spacing(2.0))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                Scrollbar::new().width(4.0).scroller_width(4.0).margin(2.0),
            ))
            .style(move |_theme, _status| theme::scrollable()),
    ]
    .spacing(7.0);
    container(pane)
        .width(280)
        .height(Length::Fill)
        .padding(iced::Padding {
            top: 12.0,
            right: 6.0,
            bottom: 12.0,
            left: 12.0,
        })
        .style(|_theme| theme::chrome())
        .into()
}

/// One note row: title, first-line snippet, link count on the right.
fn note_row<'a>(
    node: &'a super::super::knowledge_graph::Node,
    state: &'a State,
) -> Element<'a, Message> {
    let selected = state.selected == Some(node.id);
    let degree = state.graph.degree(node.id);
    let title_color = if selected { theme::STRONG } else { theme::TEXT };
    let body = row![
        column![
            label(node.title.clone(), 13.5, title_color),
            label(snippet(&node.markdown), 11.5, theme::MUTED),
        ]
        .spacing(1.0),
        Space::new().width(Length::Fill),
        label(degree.to_string(), 11.0, theme::MUTED),
    ]
    .align_y(Alignment::Center)
    .padding([4.0, 10.0]);
    let row = container(body).style(move |_theme| theme::note_row(selected, false));
    mouse_area(row)
        .on_release(Message::SelectNode(node.id))
        .on_double_click(Message::OpenNote(node.id))
        .into()
}

/// First line of a note worth showing as a preview, with inline markdown
/// markers stripped so it reads as plain text.
fn snippet(markdown: &str) -> String {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.replace("**", "").replace('`', ""))
        .unwrap_or_default()
}

/// The detail pane: title, actions, links as chips, and the note preview.
fn details(state: &State) -> Element<'_, Message> {
    let Some(node) = state.selected_node() else {
        return empty("Nothing selected", "Pick a note in the sidebar.");
    };
    let editing = state.editing == Some(node.id);
    let mut title_row: Vec<Element<'_, Message>> = vec![
        label(node.title.clone(), 21.0, theme::STRONG),
        Space::new().width(Length::Fill).into(),
    ];
    if editing {
        // While editing, the way out is "Done" — every keystroke is already
        // saved, so it just commits and returns to reading mode.
        title_row.push(accent_button("Done", Message::NoteClosed));
    } else {
        title_row.push(quiet_button("Edit note", Message::StartEditing));
    }
    title_row.push(quiet_button("Rename", Message::Rename(node.id)));
    title_row.push(quiet_button("Link…", Message::LinkClicked));
    title_row.push(danger_text_button("Delete", Message::AskDelete(node.id)));

    let neighbors: Vec<(u64, String)> = state
        .graph
        .neighbors(node.id)
        .into_iter()
        .filter_map(|id| {
            state
                .graph
                .nodes
                .iter()
                .find(|other| other.id == id)
                .map(|other| (id, other.title.clone()))
        })
        .collect();
    let links: Element<'_, Message> = if neighbors.is_empty() {
        label(
            "Not linked — press l to link this note.",
            11.5,
            theme::MUTED,
        )
    } else {
        let mut chips: Vec<Element<'_, Message>> = vec![label("Linked to", 11.5, theme::MUTED)];
        chips.extend(neighbors.iter().map(|(id, title)| {
            button(text(title.clone()).size(12.0).color(theme::TEXT))
                .on_press(Message::SelectNode(*id))
                .padding([3.0, 10.0])
                .style(move |_theme, _status| theme::chip())
                .into()
        }));
        row(chips)
            .spacing(6.0)
            .align_y(Alignment::Center)
            .wrap()
            .into()
    };
    let note: Element<'_, Message> = if editing {
        text_editor(&state.draft)
            .on_action(Message::NoteEdited)
            .font(iced::Font::MONOSPACE)
            .size(13.0)
            .style(move |_theme, _status| editor_style())
            .into()
    } else if node.markdown.trim().is_empty() {
        label("No notes yet — press e to write one.", 13.0, theme::MUTED)
    } else {
        markdown(&node.markdown, &node.title)
    };
    column![
        row(title_row).spacing(4.0).align_y(Alignment::Center),
        Space::new().height(10.0),
        links,
        Space::new().height(6.0),
        rule::horizontal(1.0).style(|_theme| rule_style()),
        Space::new().height(6.0),
        scrollable(note)
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(scrollable::Direction::Vertical(
                Scrollbar::new().width(4.0).scroller_width(4.0).margin(2.0),
            ))
            .style(move |_theme, _status| theme::scrollable()),
    ]
    .spacing(2.0)
    .into()
}

/// Quiet borderless text button.
fn quiet_button(label: &str, message: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(12.5).color(theme::MUTED))
        .on_press(message)
        .padding([6.0, 9.0])
        .style(move |_theme, status| hover_wash(status))
        .into()
}

/// A quiet button whose label carries the accent color — the primary action
/// of the moment, e.g. "Done" while editing.
fn accent_button(label: &str, message: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(12.5).color(theme::ACCENT))
        .on_press(message)
        .padding([6.0, 9.0])
        .style(move |_theme, status| hover_wash(status))
        .into()
}

fn danger_text_button(label: &str, message: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(12.5).color(theme::DANGER))
        .on_press(message)
        .padding([6.0, 9.0])
        .style(move |_theme, status| hover_wash(status))
        .into()
}

// --- markdown ---------------------------------------------------------------

/// Render a note as a reading column: headings sized, accent list markers,
/// muted quotes, mono fenced code. Inline markers are stripped, and a
/// leading H1 that just repeats the note title is dropped — the pane
/// already shows the title.
fn markdown(source: &str, title: &str) -> Element<'static, Message> {
    let mut lines: Vec<Element<'_, Message>> = Vec::new();
    let mut in_fence = false;
    for raw in source.lines() {
        let trimmed = raw.trim_start();
        let numbered = trimmed.split_once(' ').filter(|(num, _)| {
            num.len() >= 2
                && num.ends_with('.')
                && num[..num.len() - 1].chars().all(|c| c.is_ascii_digit())
        });
        let line: Element<'_, Message> = if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        } else if in_fence {
            text(raw.to_string())
                .size(12.5)
                .color(theme::TEXT)
                .font(iced::Font::MONOSPACE)
                .into()
        } else if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let text = trimmed.trim_start_matches('#').trim_start();
            if level == 1 && lines.is_empty() && text == title.trim() {
                continue;
            }
            let size = match level {
                1 => 22.0,
                2 => 18.0,
                _ => 15.5,
            };
            label(text.to_string(), size, theme::STRONG)
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            row![
                label("•", 14.0, theme::ACCENT),
                label(rest.replace("**", "").replace('`', ""), 14.0, theme::TEXT),
            ]
            .spacing(8.0)
            .into()
        } else if let Some((num, rest)) = numbered {
            row![
                label(num.to_string(), 14.0, theme::ACCENT),
                label(rest.replace("**", "").replace('`', ""), 14.0, theme::TEXT),
            ]
            .spacing(8.0)
            .into()
        } else if let Some(rest) = trimmed.strip_prefix('>') {
            label(rest.trim_start().to_string(), 14.0, theme::MUTED)
        } else {
            label(raw.replace("**", "").replace('`', ""), 14.0, theme::TEXT)
        };
        lines.push(line);
    }
    column(lines).spacing(9.0).into()
}

// --- visualize tab ----------------------------------------------------------

fn viz_tab(state: &State) -> Element<'_, Message> {
    if state.graph.nodes.is_empty() {
        return empty("Nothing to visualize", "Add notes on the Graph tab first.");
    }
    let canvas: Element<'_, Message> = super::canvas::view(&state.canvas);
    let bar = row![
        quiet_button("⟳ Shuffle", Message::ShuffleLayout),
        quiet_button("Fit", Message::FitView),
        Space::new().width(Length::Fill),
        label("drag pan · scroll zoom · click select", 11.5, theme::MUTED,),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);
    column![bar, canvas]
        .spacing(8.0)
        .padding(iced::Padding {
            top: 6.0,
            right: 14.0,
            bottom: 12.0,
            left: 14.0,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// --- dialogs ----------------------------------------------------------------

fn dialog(state: &State) -> Option<Element<'_, Message>> {
    match &state.dialog {
        Dialog::None => None,
        Dialog::TitleForm { editing, value } => {
            let field = text_input("Title", value)
                .id(super::title_input())
                .on_input(Message::TitleChanged)
                .on_submit(Message::TitleSubmitted)
                .size(13.0)
                .style(move |_theme, _status| theme::text_input());
            let save: Element<'_, Message> = button(text("Save").size(12.5).color(on_accent()))
                .on_press(Message::TitleSubmitted)
                .style(move |_theme, _status| theme::primary_button())
                .into();
            let body = column![
                label(
                    if editing.is_some() {
                        "Rename note"
                    } else {
                        "New note"
                    },
                    15.0,
                    theme::STRONG,
                ),
                field,
                row![
                    Space::new().width(Length::Fill),
                    quiet_button("Cancel", Message::DialogCancelled),
                    save
                ]
                .spacing(8.0),
            ]
            .spacing(12.0)
            .into();
            Some(card(body))
        }
        Dialog::LinkPicker { highlight } => {
            let selected_id = state.selected?;
            let toggles: Vec<Element<'_, Message>> = state
                .graph
                .nodes
                .iter()
                .filter(|node| node.id != selected_id)
                .enumerate()
                .map(|(index, node)| {
                    let linked = state.graph.is_linked(selected_id, node.id);
                    let highlighted = index == *highlight;
                    let mark: Element<'_, Message> = if linked {
                        label("✓", 13.0, theme::ACCENT)
                    } else {
                        label("  ", 13.0, theme::MUTED)
                    };
                    let title_color = if linked { theme::TEXT } else { theme::MUTED };
                    button(row![mark, label(node.title.clone(), 13.0, title_color)].spacing(10.0))
                        .on_press(Message::ToggleLink(node.id))
                        .padding([6.0, 10.0])
                        .style(move |_theme, status| match status {
                            button::Status::Hovered | button::Status::Pressed => hover_wash(status),
                            _ if highlighted => button::Style {
                                background: Some(theme::ACCENT_SOFT.into()),
                                border: iced::border::Border {
                                    radius: theme::rounded(6.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            _ => button::Style::default(),
                        })
                        .into()
                })
                .collect();
            let done: Element<'_, Message> = button(text("Done").size(12.5).color(on_accent()))
                .on_press(Message::DialogCancelled)
                .style(move |_theme, _status| theme::primary_button())
                .into();
            let body = column![
                label("Link / unlink", 15.0, theme::STRONG),
                scrollable(column(toggles).spacing(2.0))
                    .width(Length::Fill)
                    .height(Length::Fixed(320.0))
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new()
                            .width(4.0)
                            .scroller_width(4.0)
                            .margin(2.0),
                    ))
                    .style(move |_theme, _status| theme::scrollable()),
                row![Space::new().width(Length::Fill), done],
            ]
            .spacing(12.0)
            .into();
            Some(card(body))
        }
        Dialog::ConfirmDelete { .. } => {
            let delete: Element<'_, Message> = button(text("Delete").size(12.5).color(on_accent()))
                .on_press(Message::ConfirmDelete)
                .style(move |_theme, _status| theme::danger_button())
                .into();
            let body = column![
                label("Delete note", 15.0, theme::STRONG),
                label(
                    "This removes the note and all of its links.",
                    13.0,
                    theme::TEXT,
                ),
                row![
                    Space::new().width(Length::Fill),
                    quiet_button("Cancel", Message::DialogCancelled),
                    delete,
                ]
                .spacing(8.0),
            ]
            .spacing(12.0)
            .into();
            Some(card(body))
        }
    }
}

/// A dimmed modal over the whole app with a centered card. The dim layer
/// sits on top of the app in a [`Stack`] and captures clicks — on the dim
/// itself the click cancels; buttons inside the card capture first.
fn overlay<'a>(app: Element<'a, Message>, card: Element<'a, Message>) -> Element<'a, Message> {
    let dimmed = mouse_area(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_theme| theme::dimmed()),
    )
    .on_release(Message::DialogCancelled);
    iced::widget::stack![app, dimmed]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn card(body: Element<'_, Message>) -> Element<'_, Message> {
    container(body)
        .style(move |_theme| theme::card())
        .padding(20)
        .max_width(360)
        .into()
}

fn empty(title: &str, hint: &str) -> Element<'static, Message> {
    let body = column![
        label(title, 15.0, theme::MUTED),
        label(hint, 11.5, theme::MUTED),
    ]
    .spacing(4.0)
    .align_x(Alignment::Center);
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn on_accent() -> iced::Color {
    iced::Color::WHITE
}

fn rule_style() -> rule::Style {
    rule::Style {
        color: theme::LINE,
        radius: iced::border::Radius::default(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn editor_style() -> iced::widget::text_editor::Style {
    iced::widget::text_editor::Style {
        background: iced::Color::TRANSPARENT.into(),
        border: iced::border::Border::default(),
        placeholder: theme::MUTED,
        value: theme::TEXT,
        selection: theme::ACCENT_SOFT,
    }
}
