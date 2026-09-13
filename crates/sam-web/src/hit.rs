//! What a pointer means where it lands. A box that carries one of these is an
//! element with a handler on it, so the browser's own hit-testing decides what
//! was aimed at: the innermost target wins, and a box scrolled out of the pane
//! cannot be hit because it is not on screen to be hit.
//!
//! Links are the exception to boxes: a link is carried by the run of text it
//! was written on ([`crate::style::Span::link`]), so clicking a sentence that
//! mentions one only opens it where the words actually are.

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HitTarget {
    Tab(usize),
    Item(usize),
    Link(String),
    /// The reader's close button, in the pane's title row. Keys close the
    /// reader too, but a pointer had no way out of a post before this.
    Close,
    /// A card the pointer has moved onto, rather than pressed. The selection
    /// follows it the way it follows the arrow keys — one card highlighted,
    /// the one it left no longer — and nothing opens.
    Hover(usize),
}
