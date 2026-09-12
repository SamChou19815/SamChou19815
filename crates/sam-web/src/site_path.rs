use std::fmt;

/// A path into this site, like `/blog`: rooted at the domain, never carrying
/// scheme or host, and free of trailing slashes but for the root's own. A bare
/// string cannot be passed where one is wanted — a link's URL, say — without
/// [`SitePath::parse`] vetting it first.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SitePath(String);

impl SitePath {
    pub fn root() -> Self {
        SitePath(String::from("/"))
    }

    /// Wraps a `/`-rooted path this app built itself, dropping any trailing
    /// slash it somehow ends with.
    pub fn new(path: impl Into<String>) -> Self {
        let mut path = path.into();
        debug_assert!(path.starts_with('/'), "a site path is rooted: {path}");
        while path.len() > 1 && path.ends_with('/') {
            path.pop();
        }
        SitePath(path)
    }

    /// The path an outside string names, if it names one at all: it has to be
    /// `/`-rooted — anything else belongs to a host or a browser, not to a
    /// path this app could ever serve.
    pub fn parse(path: &str) -> Option<Self> {
        path.starts_with('/').then(|| Self::new(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SitePath {
    fn default() -> Self {
        Self::root()
    }
}

impl fmt::Display for SitePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
