use std::fmt;

/// A `/`-rooted path within this site. No scheme, host, or trailing slash.
#[derive(Clone, PartialEq)]
pub(crate) struct SitePath(String);

impl SitePath {
    pub(crate) fn root() -> Self {
        SitePath(String::from("/"))
    }

    pub(crate) fn new(path: impl Into<String>) -> Self {
        let mut path = path.into();
        debug_assert!(path.starts_with('/'), "a site path is rooted: {path}");
        while path.len() > 1 && path.ends_with('/') {
            path.pop();
        }
        SitePath(path)
    }

    pub(crate) fn parse(path: &str) -> Option<Self> {
        path.starts_with('/').then(|| Self::new(path))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SitePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
