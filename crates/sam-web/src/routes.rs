//! The URL's view of the app: which screen a path names, the title that goes
//! with it, and where a clicked URL leads.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::posts;
use crate::site_path::SitePath;
use crate::tab::Tab;

pub(crate) enum Screen {
    Tab(Tab),
    Post(usize),
}

impl Screen {
    pub(crate) fn tab(&self) -> Tab {
        match self {
            Screen::Tab(tab) => *tab,
            Screen::Post(_) => Tab::Blog,
        }
    }
}

pub(crate) fn screen_at(path: &SitePath) -> Option<Screen> {
    if let Some(post) = posts::find(path) {
        return Some(Screen::Post(post));
    }
    let tab = Tab::ALL
        .iter()
        .copied()
        .find(|tab| tab.route().as_str() == path.as_str())
        // Anything else under /blog (an unpublished post) lands on the index.
        .or_else(|| {
            path.as_str()
                .strip_prefix(Tab::Blog.route().as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
                .then_some(Tab::Blog)
        })?;
    Some(Screen::Tab(tab))
}

pub(crate) fn has_view(path: &SitePath) -> bool {
    screen_at(path).is_some()
}

pub(crate) const SHELL_TITLE: EncryptedString = encrypted_str!("Developer Sam — Terminal");

pub(crate) fn title_for(path: &SitePath) -> String {
    match screen_at(path) {
        Some(Screen::Post(post)) => {
            format!("{} | {}", posts::POSTS[post].title(), posts::blog_title())
        }
        Some(Screen::Tab(Tab::About)) => encrypted_str!("About | Developer Sam").to_string(),
        Some(Screen::Tab(Tab::Timeline)) => encrypted_str!("Timeline | Developer Sam").to_string(),
        Some(Screen::Tab(Tab::Help)) => encrypted_str!("Help | Developer Sam").to_string(),
        Some(Screen::Tab(Tab::Blog)) => posts::blog_title().to_string(),
        None => SHELL_TITLE.to_string(),
    }
}

pub(crate) enum LinkTarget {
    View(SitePath),
    External(String),
    /// Anything but http(s) — a `javascript:` URL would run in this document.
    Ignore,
}

/// `in_app`: at the prompt, a link to one of the app's views is just a URL
/// to open like any other.
pub(crate) fn link_target(url: &str, in_app: bool) -> LinkTarget {
    if in_app {
        if let Some(path) = site_path(url) {
            if has_view(&path) {
                return LinkTarget::View(path);
            }
        }
    }
    if starts_with_ignore_case(url, "https://") || starts_with_ignore_case(url, "http://") {
        LinkTarget::External(url.to_string())
    } else {
        LinkTarget::Ignore
    }
}

fn site_path(url: &str) -> Option<SitePath> {
    if url.starts_with('/') {
        return Some(SitePath::new(url));
    }
    let rest = strip_prefix_ignore_case(url, "https://")
        .or_else(|| strip_prefix_ignore_case(url, "http://"))?;
    let rest = strip_prefix_ignore_case(rest, "www.").unwrap_or(rest);
    let rest = strip_prefix_ignore_case(rest, "developersam.com")?;
    match rest {
        "" => Some(SitePath::root()),
        _ if rest.starts_with('/') => Some(SitePath::new(rest)),
        // e.g. `developersam.com.example.org`
        _ => None,
    }
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    starts_with_ignore_case(text, prefix).then(|| &text[prefix.len()..])
}
