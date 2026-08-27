//! Inline images, painted as truecolor half-blocks.
//!
//! Each cell is a `▀` whose foreground is the upper pixel and whose background
//! is the lower one, so a cell carries two stacked pixels and the effective
//! pixel grid is square. That is plain SGR output: it works in the native
//! binary and on the web with no graphics protocol, no xterm addon, and no
//! further patches to iocraft.
//!
//! `build.rs` bakes every asset once, at [`HERO`] resolution; smaller
//! placements box-filter that grid down here. Each image also records where it
//! landed, which the web front-end uses to lay a crisp `<img>` over the art —
//! see [`regions`].

use crate::theme;
use crossterm::style::Color;
use iocraft::prelude::*;
use std::cell::{Cell, RefCell};

/// One baked asset: `cols * 2 * rows` RGB pixels at `offset` in [`BLOB`].
/// `len == 0` marks a dimensions-only entry — the blog's images, whose pixels
/// are never baked. The terminal draws a captioned placeholder frame there and
/// the web overlay lays the real, full-resolution file over the reported
/// region, exactly as it does for a baked image.
struct Baked {
    url: &'static str,
    cols: u16,
    rows: u16,
    offset: usize,
    len: usize,
}

include!(concat!(env!("OUT_DIR"), "/images.rs"));

/// Card and project-row thumbnails.
pub const THUMBNAIL: (u16, u16) = (32, 8);
/// The detail dialog's hero, and the size everything is baked at.
pub const HERO: (u16, u16) = (56, 16);
/// The portrait beside the About pane's program.
pub const AVATAR: (u16, u16) = (26, 13);

/// The About pane's portrait, as on the homepage.
pub const PORTRAIT: &str = "/sam-by-megan-3-square.webp";

/// Narrower than this and the text layout has nothing to spare, so images are
/// dropped rather than squeezed.
const MIN_COLS: u16 = 60;

/// Which stacking layer an image sits on. The canvas paints the open dialog
/// over the pane and the cards behind it simply stop being visible, but the web
/// overlay is a flat list of `<img>` with no such ordering — so it has to be
/// told. Only the frame's topmost layer is reported, which is why a dialog with
/// no artwork of its own still hides every card thumbnail behind it.
pub const LAYER_PANE: u8 = 0;
pub const LAYER_DIALOG: u8 = 1;

/// Whether images are drawn at a given terminal width. Layout and scroll math
/// both route through this, so they can never disagree about a card's height.
pub fn enabled(cols: u16) -> bool {
    // 0 is the pre-resize placeholder, which `crate::content_width` reads as a
    // comfortable 80 columns; images are on there too.
    cols == 0 || cols >= MIN_COLS
}

fn baked(url: &str) -> Option<&'static Baked> {
    INDEX.iter().find(|baked| baked.url == url)
}

/// Cells `url` occupies when fitted into `bounds`, preserving aspect. `None`
/// when nothing is baked under that name.
pub fn size(url: &str, (max_cols, max_rows): (u16, u16)) -> Option<(u16, u16)> {
    let baked = baked(url)?;
    // Never upscale past what was baked — there are no more pixels to show.
    let scale = f64::min(
        f64::from(max_cols) / f64::from(baked.cols),
        f64::from(max_rows) / f64::from(baked.rows),
    )
    .min(1.0);
    let cols = (f64::from(baked.cols) * scale).round() as u16;
    let rows = (f64::from(baked.rows) * scale).round() as u16;
    Some((cols.max(1), rows.max(1)))
}

/// Rows an optional image contributes to a row of a list, at `cols` wide.
/// Zero when there is no image or the terminal is too narrow — the single
/// answer both the view and the height functions use.
pub fn rows(url: Option<&str>, cols: u16, bounds: (u16, u16)) -> usize {
    if !enabled(cols) {
        return 0;
    }
    url.and_then(|url| size(url, bounds))
        .map_or(0, |(_, rows)| usize::from(rows))
}

/// Box-filters a baked grid down to `cols * 2 * rows` RGB pixels.
fn sample(baked: &Baked, cols: u16, rows: u16) -> Vec<[u8; 3]> {
    let source = &BLOB[baked.offset..baked.offset + baked.len];
    let (source_width, source_height) = (usize::from(baked.cols), usize::from(baked.rows) * 2);
    let (width, height) = (usize::from(cols), usize::from(rows) * 2);
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let y0 = y * source_height / height;
        let y1 = (((y + 1) * source_height).div_ceil(height))
            .min(source_height)
            .max(y0 + 1);
        for x in 0..width {
            let x0 = x * source_width / width;
            let x1 = (((x + 1) * source_width).div_ceil(width))
                .min(source_width)
                .max(x0 + 1);
            let (mut red, mut green, mut blue, mut count) = (0u32, 0u32, 0u32, 0u32);
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy * source_width + sx) * 3;
                    red += u32::from(source[at]);
                    green += u32::from(source[at + 1]);
                    blue += u32::from(source[at + 2]);
                    count += 1;
                }
            }
            pixels.push([
                (red / count) as u8,
                (green / count) as u8,
                (blue / count) as u8,
            ]);
        }
    }
    pixels
}

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb { r, g, b }
}

// --- Where each image landed, for the web overlay -----------------------------

/// A drawn image's cell rectangle, in canvas coordinates. The app runs in the
/// alternate screen, so canvas row 0 is viewport row 0 and the host needs no
/// offset to place an `<img>` over it.
///
/// `visible_*` is the part that survived the pane's clipping. A card scrolled
/// half off the bottom paints only part of its artwork, and the overlay has to
/// crop to the same rectangle or the picture spills over the status bar. It is
/// measured, not predicted: [`CanvasSubviewMut::cell`] returns `None` outside
/// the clip region, so the draw loop learns each cell's fate as it writes it.
///
/// `x`/`y` is where the whole picture starts, and goes negative when it is
/// scrolled part-way off the top of a pane — the reader pulls the block its
/// viewport begins inside up out of sight. The overlay lays the full-resolution
/// file over that origin and crops it to `visible_*`, so it needs the origin of
/// the picture rather than of the part that survived.
#[derive(Clone, Copy)]
pub struct Region {
    pub url: &'static str,
    pub x: i16,
    pub y: i16,
    pub cols: u16,
    pub rows: u16,
    pub visible_x: i16,
    pub visible_y: i16,
    pub visible_cols: u16,
    pub visible_rows: u16,
    pub layer: u8,
}

thread_local! {
    static REGIONS: RefCell<Vec<Region>> = const { RefCell::new(Vec::new()) };
    /// The topmost layer this frame draws, set by `Root` before anything paints.
    static TOP_LAYER: Cell<u8> = const { Cell::new(LAYER_PANE) };
}

/// Drops the previous frame's rects. Called from `Root` at the top of the
/// update pass, which iocraft always runs to completion — update, then draw,
/// then write — inside a single host poll. So the host only ever observes a
/// whole frame's worth, and a card that scrolled away leaves no ghost behind.
pub fn begin_frame(top_layer: u8) {
    REGIONS.with(|regions| regions.borrow_mut().clear());
    TOP_LAYER.with(|layer| layer.set(top_layer));
}

fn record(region: Region) {
    REGIONS.with(|regions| regions.borrow_mut().push(region));
}

/// Every image drawn in the current frame.
/// Every image with something on screen in the current frame. Fully clipped
/// ones are left out, so the host never mounts an `<img>` for a card that
/// scrolled away.
pub fn regions() -> Vec<Region> {
    REGIONS.with(|regions| {
        regions
            .borrow()
            .iter()
            .filter(|region| region.layer == TOP_LAYER.with(Cell::get))
            .filter(|region| region.visible_cols > 0 && region.visible_rows > 0)
            .map(|region| Region { ..*region })
            .collect()
    })
}

// --- The component -------------------------------------------------------------

#[derive(Props, Default)]
pub struct ImageProps {
    /// Site-root-relative asset path, e.g. `/timeline/flow.webp`.
    pub url: &'static str,
    /// The cell box to fit inside; one of [`THUMBNAIL`], [`HERO`], [`AVATAR`].
    pub bounds: (u16, u16),
    /// [`LAYER_PANE`] by default; [`LAYER_DIALOG`] for artwork inside a dialog.
    pub layer: u8,
    /// Caption for the placeholder frame drawn when no pixels were baked.
    pub alt: String,
}

/// Draws a baked image as half-blocks, or a captioned frame when the asset is
/// dimensions-only. Implemented against `Component` directly rather than
/// through `#[component]`, because painting per-cell foreground *and*
/// background needs the canvas, which only `draw` sees.
#[derive(Default)]
pub struct Image {
    url: &'static str,
    cols: u16,
    rows: u16,
    layer: u8,
    alt: String,
}

impl Component for Image {
    type Props<'a> = ImageProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        self.url = props.url;
        self.layer = props.layer;
        self.alt = props.alt.clone();
        let (cols, rows) = size(props.url, props.bounds).unwrap_or((0, 0));
        self.cols = cols;
        self.rows = rows;
        updater.set_layout_style(taffy::style::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::Length(f32::from(cols)),
                height: taffy::style::Dimension::Length(f32::from(rows)),
            },
            flex_shrink: 0.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let Some(baked) = baked(self.url) else {
            return;
        };
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let position = drawer.canvas_position();
        let width = usize::from(self.cols);
        let mut canvas = drawer.canvas();
        // `CanvasTextStyle` is `#[non_exhaustive]`, so it is built by mutation
        // rather than by a struct literal.
        let mut style = CanvasTextStyle::default();
        // The clipped region is a rectangle, so tracking its corners is enough.
        let (mut first, mut last) = (None, (0usize, 0usize));
        if baked.len == 0 {
            // Dimensions only: probe every cell so `first`/`last` measure the
            // clipped rectangle exactly as the pixel path does, then draw the
            // placeholder the web overlay's `<img>` replaces.
            for row in 0..usize::from(self.rows) {
                for col in 0..width {
                    if canvas.cell(col as isize, row as isize).is_some() {
                        first.get_or_insert((col, row));
                        last = (last.0.max(col), row);
                    }
                }
            }
            let rows = usize::from(self.rows);
            let framed = rows >= 3;
            let margin = usize::from(framed);
            let label = if self.alt.is_empty() {
                "[image]".to_string()
            } else {
                format!("[image: {}]", self.alt)
            };
            let caption =
                crate::markdown::truncate(&label, width.saturating_sub(2 * margin).max(1));
            let caption_row = rows / 2;
            let start =
                margin + (width.saturating_sub(2 * margin).max(1) - caption.chars().count()) / 2;
            for row in 0..rows {
                for col in 0..width {
                    let glyph = if framed {
                        frame_glyph(col, row, width, rows)
                    } else {
                        ""
                    };
                    if glyph.is_empty() {
                        continue;
                    }
                    style.color = Some(theme::BORDER);
                    canvas.set_text(col as isize, row as isize, glyph, style);
                }
            }
            style.color = Some(theme::MUTED);
            for (offset, glyph) in caption.char_indices() {
                let piece = &caption[offset..offset + glyph.len_utf8()];
                canvas.set_text(
                    (start + offset) as isize,
                    caption_row as isize,
                    piece,
                    style,
                );
            }
        } else {
            let pixels = sample(baked, self.cols, self.rows);
            for row in 0..usize::from(self.rows) {
                for col in 0..width {
                    if canvas.cell(col as isize, row as isize).is_some() {
                        first.get_or_insert((col, row));
                        last = (last.0.max(col), row);
                    }
                    let upper = pixels[row * 2 * width + col];
                    let lower = pixels[(row * 2 + 1) * width + col];
                    canvas.set_background_color(col as isize, row as isize, 1, 1, color(lower));
                    style.color = Some(color(upper));
                    canvas.set_text(col as isize, row as isize, "▀", style);
                }
            }
        }
        let (x, y) = (position.x, position.y);
        let (visible_x, visible_y, visible_cols, visible_rows) = match first {
            Some((col, row)) => (
                x + col as i16,
                y + row as i16,
                (last.0 + 1 - col) as u16,
                (last.1 + 1 - row) as u16,
            ),
            None => (x, y, 0, 0),
        };
        record(Region {
            url: self.url,
            x,
            y,
            cols: self.cols,
            rows: self.rows,
            visible_x,
            visible_y,
            visible_cols,
            visible_rows,
            layer: self.layer,
        });
    }
}

/// The box-drawing glyph for one cell of a placeholder's frame; the empty
/// string for the interior.
fn frame_glyph(col: usize, row: usize, cols: usize, rows: usize) -> &'static str {
    if row == 0 {
        if col == 0 {
            "┌"
        } else if col + 1 == cols {
            "┐"
        } else {
            "─"
        }
    } else if row + 1 == rows {
        if col == 0 {
            "└"
        } else if col + 1 == cols {
            "┘"
        } else {
            "─"
        }
    } else if col == 0 || col + 1 == cols {
        "│"
    } else {
        ""
    }
}
