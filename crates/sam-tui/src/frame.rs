//! The top of a frame, as one named thing.
//!
//! Two registries are filled in as the tree paints — where each image landed
//! ([`crate::image`]) and where each click region is ([`crate::hit`]) — and both
//! describe one frame only. A region that outlives what drew it answers for
//! something no longer on screen: the cards of a tab that has since been
//! switched away used to take clicks on the pane that replaced them, and an
//! image the host was never told about goes on floating over whatever came
//! next.
//!
//! So both are dropped at the same instant, from the outermost component, in
//! the draw pass — before anything has painted into the new frame and after
//! everything has finished reading the old one.

use crate::{hit, image};
use iocraft::prelude::*;

/// Drops what the previous frame recorded, just before this one paints.
pub trait UseFrame {
    /// `top_layer` is the image layer the host should mount this frame; see
    /// [`image::begin_frame`].
    fn use_frame(&mut self, top_layer: u8);
}

impl UseFrame for Hooks<'_, '_> {
    fn use_frame(&mut self, top_layer: u8) {
        self.use_hook(|| FrameHook {
            top_layer: image::LAYER_PANE,
        })
        .top_layer = top_layer;
    }
}

struct FrameHook {
    top_layer: u8,
}

impl Hook for FrameHook {
    /// The outermost component's hooks run before any of its descendants draw,
    /// so this is the first thing that happens in a frame.
    fn pre_component_draw(&mut self, _drawer: &mut ComponentDrawer) {
        hit::clear();
        image::begin_frame(self.top_layer);
    }
}
