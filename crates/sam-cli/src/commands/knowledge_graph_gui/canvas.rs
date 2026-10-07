//! The graph canvas: an `iced` [`canvas::Program`] — pure drawing plus mouse
//! events that come back as messages.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Style, Text};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::theme;
use super::Message;

/// Longest title shown next to a dot before truncation with `…`.
const LABEL_CHARS: usize = 20;

pub struct NodePoint {
    pub id: u64,
    pub title: String,
    pub x: f64,
    pub y: f64,
}

/// Drag/click bookkeeping the canvas runtime keeps per instance.
#[derive(Default)]
pub struct Interaction {
    dragging: Option<Point>,
    pressed_at: Option<(Point, std::time::Instant)>,
}

#[derive(Default)]
pub struct GraphCanvas {
    nodes: Vec<NodePoint>,
    edges: Vec<(usize, usize)>,
    selected: Option<usize>,
    view: ViewState,
    content: canvas::Cache,
    grid: canvas::Cache,
}

#[derive(Clone, Copy)]
struct ViewState {
    pan: Vector,
    zoom: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            pan: Vector::default(),
            zoom: 1.0,
        }
    }
}

impl GraphCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Swap in new graph data (after any structural change).
    pub fn set_graph(
        &mut self,
        nodes: Vec<NodePoint>,
        edges: Vec<(usize, usize)>,
        selected: Option<usize>,
    ) {
        self.nodes = nodes;
        self.edges = edges;
        self.selected = selected;
        self.view = ViewState::default();
        self.content.clear();
        self.grid.clear();
    }

    pub fn pan(&mut self, delta: Vector) {
        self.view.pan += delta;
        self.content.clear();
        self.grid.clear();
    }

    pub fn zoom(&mut self, factor: f32, at: Option<Point>) {
        let old = self.view.zoom;
        self.view.zoom = (old * factor).clamp(0.15, 10.0);
        let ratio = self.view.zoom / old;
        if let Some(at) = at {
            self.view.pan = Vector::new(
                at.x - (at.x - self.view.pan.x) * ratio,
                at.y - (at.y - self.view.pan.y) * ratio,
            );
        } else {
            self.view.pan *= ratio;
        }
        self.content.clear();
        self.grid.clear();
    }

    pub fn reset_view(&mut self) {
        self.view = ViewState::default();
        self.content.clear();
        self.grid.clear();
    }

    /// World-space bounding box of the nodes, with padding so labels fit.
    fn bounds(&self) -> Rectangle<f32> {
        if self.nodes.is_empty() {
            return Rectangle {
                x: -0.5,
                y: -0.5,
                width: 1.0,
                height: 1.0,
            };
        }
        let min_x = self
            .nodes
            .iter()
            .map(|n| n.x as f32)
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .nodes
            .iter()
            .map(|n| n.x as f32)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .nodes
            .iter()
            .map(|n| n.y as f32)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .nodes
            .iter()
            .map(|n| n.y as f32)
            .fold(f32::NEG_INFINITY, f32::max);
        Rectangle {
            x: min_x - 0.1,
            y: min_y - 0.15,
            width: (max_x - min_x + 0.45).max(0.85),
            height: (max_y - min_y + 0.3).max(0.85),
        }
    }

    /// Fit-the-bounds transform, in canvas-local coordinates.
    fn to_local(&self, bounds: Rectangle<f32>) -> Box<dyn Fn(&NodePoint) -> Point + '_> {
        let world = self.bounds();
        let scale = ((bounds.width - 60.0) / world.width.max(1e-3))
            .min((bounds.height - 60.0) / world.height.max(1e-3))
            .max(1e-3)
            * self.view.zoom;
        let center = bounds.center();
        let world_center = world.center();
        Box::new(move |node: &NodePoint| {
            Point::new(
                center.x + (node.x as f32 - world_center.x) * scale,
                center.y - (node.y as f32 - world_center.y) * scale,
            ) + self.view.pan
        })
    }

    /// Inverse of [`to_local`] for hit-testing (approximate: search by
    /// forward projection instead).
    fn hit(&self, bounds: Rectangle<f32>, at: Point) -> Option<usize> {
        let to_local = self.to_local(bounds);
        let mut best: Option<(f32, usize)> = None;
        for (index, node) in self.nodes.iter().enumerate() {
            let dot = to_local(node);
            let dx = at.x - dot.x;
            let dy = at.y - dot.y;
            let label_len = truncate(&node.title, LABEL_CHARS).chars().count() as f32 * 7.0;
            let dx = if dy.abs() <= 10.0 && (-8.0..=label_len + 10.0).contains(&dx) {
                0.0
            } else {
                dx
            };
            let score = dx * dx + dy * dy;
            if best.is_none_or(|(best_score, _)| score < best_score) {
                best = Some((score, index));
            }
        }
        best.filter(|(score, _)| *score <= 900.0).map(|(_, i)| i)
    }
}

/// The view function for the canvas widget.
pub fn view(canvas: &GraphCanvas) -> Element<'_, Message> {
    Canvas::new(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

impl canvas::Program<Message> for GraphCanvas {
    type State = Interaction;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle<f32>,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let grid = self.grid.draw(renderer, bounds.size(), |frame| {
            draw_grid(frame, self.view.pan);
        });
        let content = self.content.draw(renderer, bounds.size(), |frame| {
            draw_graph(frame, self, bounds);
        });
        vec![grid, content]
    }

    fn update(
        &self,
        interaction: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle<f32>,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::widget::Action;
        let position = cursor.position_in(bounds)?;
        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let factor = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        (y * 0.75).exp()
                    }
                };
                Some(Action::publish(Message::Zoom {
                    factor: if factor.is_finite() { factor } else { 1.0 },
                    at: Some(position),
                }))
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                interaction.dragging = Some(position);
                interaction.pressed_at = Some((position, std::time::Instant::now()));
                Some(Action::capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => match interaction.dragging {
                Some(origin) if origin.distance(position) > 1.0 => {
                    let delta = Vector::new(position.x - origin.x, position.y - origin.y);
                    interaction.dragging = Some(position);
                    interaction.pressed_at = None;
                    Some(Action::publish(Message::Pan(delta)))
                }
                _ => None,
            },
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let was_click = interaction.pressed_at.is_some_and(|(at, when)| {
                    at.distance(position) < 4.0
                        && when.elapsed() < std::time::Duration::from_secs(1)
                });
                interaction.dragging = None;
                interaction.pressed_at = None;
                if was_click {
                    if let Some(index) = self.hit(bounds, position) {
                        return Some(Action::publish(Message::SelectNode(self.nodes[index].id)));
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle<f32>,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_grid(frame: &mut Frame, pan: Vector) {
    let size = frame.size();
    let step = 26.0;
    let mut x = -pan.x.rem_euclid(step);
    while x < size.width {
        let mut y = -pan.y.rem_euclid(step);
        while y < size.height {
            frame.fill_rectangle(
                Point::new(x - 0.5, y - 0.5),
                Size::new(1.0, 1.0),
                theme::GRID,
            );
            y += step;
        }
        x += step;
    }
}

fn draw_graph(frame: &mut Frame, canvas: &GraphCanvas, bounds: Rectangle<f32>) {
    // Draw in the full canvas-local space; `to_local` already accounts for
    // the widget bounds centering.
    let to_local = canvas.to_local(bounds);
    for (a, b) in &canvas.edges {
        let touches = canvas.selected == Some(*a) || canvas.selected == Some(*b);
        let stroke = canvas::Stroke {
            style: Style::Solid(if touches { theme::ACCENT } else { theme::EDGE }),
            width: if touches { 2.0 } else { 1.25 },
            ..Default::default()
        };
        frame.stroke(
            &Path::line(to_local(&canvas.nodes[*a]), to_local(&canvas.nodes[*b])),
            stroke,
        );
    }
    for (index, node) in canvas.nodes.iter().enumerate() {
        let dot = to_local(node);
        let is_selected = canvas.selected == Some(index);
        let radius = if is_selected { 6.0 } else { 4.5 };
        if is_selected {
            frame.fill(&Path::circle(dot, radius + 7.0), theme::HALO);
        }
        frame.fill(
            &Path::circle(dot, radius),
            if is_selected {
                theme::ACCENT
            } else {
                theme::MUTED
            },
        );
        frame.fill_text(Text {
            content: truncate(&node.title, LABEL_CHARS),
            position: dot + Vector::new(radius + 6.0, 0.0),
            color: if is_selected {
                theme::STRONG
            } else {
                theme::TEXT
            },
            size: 13.0.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
    }
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
