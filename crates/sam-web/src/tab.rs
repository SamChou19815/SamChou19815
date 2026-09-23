use crate::crypt::{encrypted_str, EncryptedString};
use crate::site_path::SitePath;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    About,
    Timeline,
    Blog,
}

impl Tab {
    /// Header order, also the order of the 1-3 shortcuts.
    pub(crate) const ALL: [Tab; 3] = [Tab::About, Tab::Timeline, Tab::Blog];

    /// Index into [`Tab::ALL`].
    pub(crate) const fn index(self) -> usize {
        match self {
            Tab::About => 0,
            Tab::Timeline => 1,
            Tab::Blog => 2,
        }
    }

    pub(crate) fn name(self) -> EncryptedString {
        match self {
            Tab::About => encrypted_str!("About"),
            Tab::Timeline => encrypted_str!("Timeline"),
            Tab::Blog => encrypted_str!("Blog"),
        }
    }

    /// Must match the route table in [`crate::ui`].
    pub(crate) fn route(self) -> SitePath {
        SitePath::new(
            match self {
                Tab::About => encrypted_str!("/about"),
                Tab::Timeline => encrypted_str!("/timeline"),
                Tab::Blog => encrypted_str!("/blog"),
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
        Tab::ALL
            .iter()
            .cycle()
            .nth(self.index() + by)
            .copied()
            .unwrap_or(self)
    }
}
