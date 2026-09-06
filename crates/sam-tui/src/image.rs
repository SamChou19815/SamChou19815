//! Inline images: a cell box reserved for the web front-end to fill.
//!
//! The TUI never draws artwork itself, not even a placeholder: anything painted
//! in the box would show through an image with transparency. It works out how
//! many cells a picture takes and records where the box landed; the host reads
//! [`regions`] and lays the real, full-resolution file over it — see
//! `packages/www/src/app/terminal/artwork.ts`.
//!
//! So nothing but a width and a height ever reaches the wasm binary: `build.rs`
//! records each asset's size in cells, at [`HERO`] scale, and smaller
//! placements scale that down here.

use iocraft::prelude::*;
use std::cell::{Cell, RefCell};

/// One asset's size in cells, fitted to [`HERO`]: the aspect ratio every
/// placement scales down from.
struct Asset {
    url: &'static str,
    cols: u16,
    rows: u16,
}

include!(concat!(env!("OUT_DIR"), "/images.rs"));

/// Card and project-row thumbnails.
pub const THUMBNAIL: (u16, u16) = (32, 8);
/// The detail dialog's hero, and the box every recorded size is fitted to.
pub const HERO: (u16, u16) = (56, 16);

/// Which stacking layer an image sits on. The canvas paints the open dialog
/// over the pane and the cards behind it simply stop being visible, but the web
/// overlay is a flat list of `<img>` with no such ordering — so it has to be
/// told. Only the frame's topmost layer is reported, which is why a dialog with
/// no artwork of its own still hides every card thumbnail behind it.
pub const LAYER_PANE: u8 = 0;
pub const LAYER_DIALOG: u8 = 1;

/// The box an image may fill inside a column `width` cells wide: the bounds
/// the design asks for, narrowed to the column whenever the column is the
/// smaller of the two. A phone is narrower than every box in this module, so
/// this is what puts pictures on one — scaled down rather than dropped.
///
/// Layout and scroll math both route through the bounds this returns, so they
/// can never disagree about how many rows a picture takes.
pub fn fit_width(width: usize, (max_cols, max_rows): (u16, u16)) -> (u16, u16) {
    let width = u16::try_from(width).unwrap_or(u16::MAX);
    (max_cols.min(width).max(1), max_rows)
}

/// A card's thumbnail box at a given terminal width.
pub fn thumbnail_bounds(cols: u16) -> (u16, u16) {
    fit_width(crate::content_width(cols), THUMBNAIL)
}

/// The artwork box inside a post: the blog's column, at most [`HERO`].
pub fn reader_bounds(cols: u16) -> (u16, u16) {
    fit_width(crate::blog_column_width(cols), HERO)
}

fn asset(path: &crate::site_path::SitePath) -> Option<&'static Asset> {
    ASSETS.iter().find(|asset| asset.url == path.as_str())
}

/// Cells the image at `path` occupies when fitted into `bounds`, preserving
/// aspect. `None` when the site serves no asset under that name.
pub fn size(
    path: &crate::site_path::SitePath,
    (max_cols, max_rows): (u16, u16),
) -> Option<(u16, u16)> {
    let asset = asset(path)?;
    // Never past HERO, the largest box the design lays out. Recorded sizes
    // already sit inside it, so this only guards a caller asking for more.
    let scale = f64::min(
        f64::from(max_cols) / f64::from(asset.cols),
        f64::from(max_rows) / f64::from(asset.rows),
    )
    .min(1.0);
    let cols = (f64::from(asset.cols) * scale).round() as u16;
    let rows = (f64::from(asset.rows) * scale).round() as u16;
    Some((cols.max(1), rows.max(1)))
}

/// Rows an optional image contributes to a row of a list, within `bounds`.
/// Zero when there is no image under that name — the single answer both the
/// view and the height functions use.
pub fn rows(path: Option<&crate::site_path::SitePath>, bounds: (u16, u16)) -> usize {
    path.and_then(|path| size(path, bounds))
        .map_or(0, |(_, rows)| usize::from(rows))
}

// --- Where each image landed, for the web overlay -----------------------------

/// A drawn image's cell rectangle, in canvas coordinates. The app runs in the
/// alternate screen, so canvas row 0 is viewport row 0 and the host needs no
/// offset to place an `<img>` over it.
///
/// `visible_*` is the part that survived the pane's clipping. A card scrolled
/// half off the bottom paints only part of its box, and the overlay has to crop
/// to the same rectangle or the picture spills over the status bar. It is
/// measured, not predicted: [`CanvasSubviewMut::cell`] returns `None` outside
/// the clip region, so the draw loop reads each cell's fate off the canvas.
///
/// `x`/`y` is where the whole picture starts, and goes negative when it is
/// scrolled part-way off the top of a pane — the reader pulls the block its
/// viewport begins inside up out of sight. The overlay lays the full-resolution
/// file over that origin and crops it to `visible_*`, so it needs the origin of
/// the picture rather than of the part that survived.
#[derive(Clone)]
pub struct Region {
    /// Site-root-relative asset path. Owned rather than `&'static str`: a
    /// timeline card's path lives encrypted in [`crate::data`] and only exists
    /// as text once it has been decrypted.
    pub url: crate::site_path::SitePath,
    pub x: i16,
    pub y: i16,
    pub cols: u16,
    pub rows: u16,
    pub visible_x: i16,
    pub visible_y: i16,
    pub visible_cols: u16,
    pub visible_rows: u16,
}

impl Region {
    /// What the pane's clipping took off each side, in cells, as
    /// `(top, right, bottom, left)`. The overlay crops the full-resolution file
    /// by exactly this, so a card scrolled half off the bottom does not spill
    /// its artwork over the status bar.
    pub fn insets(&self) -> (u16, u16, u16, u16) {
        let (x, y) = (i32::from(self.x), i32::from(self.y));
        let (right_edge, bottom_edge) = (x + i32::from(self.cols), y + i32::from(self.rows));
        let (visible_x, visible_y) = (i32::from(self.visible_x), i32::from(self.visible_y));
        let visible_right = visible_x + i32::from(self.visible_cols);
        let visible_bottom = visible_y + i32::from(self.visible_rows);
        let side = |value: i32| u16::try_from(value.max(0)).unwrap_or(u16::MAX);
        (
            side(visible_y - y),
            side(right_edge - visible_right),
            side(bottom_edge - visible_bottom),
            side(visible_x - x),
        )
    }
}

thread_local! {
    static REGIONS: RefCell<Vec<Region>> = const { RefCell::new(Vec::new()) };
    /// The topmost layer this frame draws, set by `Root` before anything paints.
    static TOP_LAYER: Cell<u8> = const { Cell::new(LAYER_PANE) };
}

/// Drops the previous frame's rects, so the host only ever observes a whole
/// frame's worth and a card that scrolled away leaves no ghost behind. Called
/// by [`crate::frame`] before the frame paints — and by hand on the way out of
/// the app, when there is no next frame to do it.
pub fn begin_frame(top_layer: u8) {
    REGIONS.with(|regions| regions.borrow_mut().clear());
    TOP_LAYER.with(|layer| layer.set(top_layer));
}

fn record(region: Region) {
    REGIONS.with(|regions| regions.borrow_mut().push(region));
}

/// Every image with something on screen in the current frame. Fully clipped
/// ones are left out, so the host never mounts an `<img>` for a card that
/// scrolled away.
pub fn regions() -> Vec<Region> {
    REGIONS.with(|regions| {
        regions
            .borrow()
            .iter()
            .filter(|region| region.visible_cols > 0 && region.visible_rows > 0)
            .cloned()
            .collect()
    })
}

// --- The component -------------------------------------------------------------

#[derive(Props, Default)]
pub struct ImageProps {
    pub url: crate::site_path::SitePath,
    /// The cell box to fit inside; one of [`THUMBNAIL`] or [`HERO`].
    pub bounds: (u16, u16),
    /// [`LAYER_PANE`] by default; [`LAYER_DIALOG`] for artwork inside a dialog.
    pub layer: u8,
}

/// Reserves an image's cells and reports the rectangle the host covers.
/// Implemented against `Component` directly rather than through
/// `#[component]`, because it has to measure how much of itself the pane's
/// clipping let through, which only the canvas `draw` sees can tell it.
#[derive(Default)]
pub struct Image {
    url: crate::site_path::SitePath,
    cols: u16,
    rows: u16,
    layer: u8,
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
        self.url = props.url.clone();
        self.layer = props.layer;
        let (cols, rows) = size(&props.url, props.bounds).unwrap_or((0, 0));
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
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        // Only the frame's top layer reports: the flat `<img>` overlay has no
        // stacking of its own, so a dialog open over the pane hides every card
        // thumbnail behind it.
        if self.layer != TOP_LAYER.with(Cell::get) {
            return;
        }
        let position = drawer.canvas_position();
        let (width, height) = (usize::from(self.cols), usize::from(self.rows));
        let canvas = drawer.canvas();
        // The clipped region is a rectangle, so tracking its corners is enough.
        let (mut first, mut last) = (None, (0usize, 0usize));
        for row in 0..height {
            for col in 0..width {
                if canvas.cell(col as isize, row as isize).is_some() {
                    first.get_or_insert((col, row));
                    last = (last.0.max(col), row);
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
            url: self.url.clone(),
            x,
            y,
            cols: self.cols,
            rows: self.rows,
            visible_x,
            visible_y,
            visible_cols,
            visible_rows,
        });
    }
}
