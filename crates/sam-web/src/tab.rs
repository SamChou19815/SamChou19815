//! The app's four tabs.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::site_path::SitePath;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    About,
    Timeline,
    Blog,
    Help,
}

impl Tab {
    /// Every tab, in the order the header lists and `1`–`4` reach them.
    pub(crate) const ALL: [Tab; 4] = [Tab::About, Tab::Timeline, Tab::Blog, Tab::Help];

    /// Where the tab sits in [`Tab::ALL`]: the digit key that reaches it, and
    /// the index into the per-tab state.
    pub(crate) const fn index(self) -> usize {
        match self {
            Tab::About => 0,
            Tab::Timeline => 1,
            Tab::Blog => 2,
            Tab::Help => 3,
        }
    }

    pub(crate) fn name(self) -> EncryptedString {
        match self {
            Tab::About => encrypted_str!("About"),
            Tab::Timeline => encrypted_str!("Timeline"),
            Tab::Blog => encrypted_str!("Blog"),
            Tab::Help => encrypted_str!("Help"),
        }
    }

    /// Mirrored by the route table in [`crate::ui`].
    pub(crate) fn route(self) -> SitePath {
        SitePath::new(
            match self {
                Tab::About => encrypted_str!("/about"),
                Tab::Timeline => encrypted_str!("/timeline"),
                Tab::Blog => encrypted_str!("/blog"),
                Tab::Help => encrypted_str!("/help"),
            }
            .decrypt(),
        )
    }

    pub(crate) fn next(self) -> Tab {
        self.step(1)
    }

    pub(crate) fn prev(self) -> Tab {
        self.step(Tab::ALL.len() - 1)
    }

    fn step(self, by: usize) -> Tab {
        Tab::ALL[(self.index() + by) % Tab::ALL.len()]
    }
}
