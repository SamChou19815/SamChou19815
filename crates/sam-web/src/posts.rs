//! The blog index and post bodies, generated from packages/www by build.rs.
//!
//! The corpus ships encrypted, so it is no more legible in the binary than
//! [`crate::data`] is. The encrypting happens in build.rs rather than through
//! `encrypted_str!`: a post body is far too long for rustc to const-evaluate.
//!
//! Every string is one run of a single ciphertext blob. A [`Post`] records
//! only where its strings sit ([`EncryptedRun`]) and the accessors below
//! resolve those against [`POSTS_BLOB`] when read: a `static` table holding
//! slices of the blob would carry a second copy of the whole corpus.

use crate::crypt::{EncryptedRun, EncryptedString};
use crate::site_path::SitePath;

/// The palette a highlighted span of code is painted with: the homepage's
/// Prism light tokens, the same ones [`crate::highlight`] paints samlang
/// with. The names are what build.rs's tree-sitter captures resolve to.
#[derive(Clone, Copy)]
pub(crate) enum SpanColor {
    Keyword,
    String,
    Number,
    Function,
    Type,
    Comment,
}

/// One highlighted range of a code block's line: where in the line it starts,
/// how long it is, and the color that paints it.
pub(crate) struct CodeSpan {
    pub(crate) start: u32,
    pub(crate) len: u32,
    pub(crate) color: SpanColor,
}

/// The tree-sitter highlighting build.rs compiled for one fenced block: per
/// line, its spans.
pub(crate) struct CodeBlock {
    pub(crate) lines: &'static [&'static [CodeSpan]],
}

pub(crate) struct Post {
    title: EncryptedRun,
    year: EncryptedRun,
    month: EncryptedRun,
    date: EncryptedRun,
    /// Empty for external posts, which have no page on this site.
    slug: EncryptedRun,
    external_url: Option<EncryptedRun>,
    /// Markdown body with the frontmatter stripped. Empty for external posts.
    body: EncryptedRun,
    /// One entry per fenced block in the body, in order: its compiled
    /// highlighting, or no lines for the blocks that get none — plain ones,
    /// and the samlang ones the runtime highlights itself.
    code_blocks: &'static [CodeBlock],
}

include!(concat!(env!("OUT_DIR"), "/posts.rs"));

/// The blog's name.
pub(crate) fn blog_title() -> EncryptedString {
    BLOG_TITLE.of(POSTS_BLOB)
}

impl Post {
    pub(crate) fn title(&self) -> EncryptedString {
        self.title.of(POSTS_BLOB)
    }

    pub(crate) fn body(&self) -> EncryptedString {
        self.body.of(POSTS_BLOB)
    }

    pub(crate) fn code_blocks(&self) -> &'static [CodeBlock] {
        self.code_blocks
    }

    pub(crate) fn is_external(&self) -> bool {
        self.external_url.is_some()
    }

    pub(crate) fn formatted_date(&self) -> String {
        format!(
            "{}-{}-{}",
            self.year.of(POSTS_BLOB),
            self.month.of(POSTS_BLOB),
            self.date.of(POSTS_BLOB)
        )
    }

    /// The post's permalink as a site path. Only local posts have one; an
    /// external post's empty slug makes this meaningless, which is why
    /// [`find`] never matches one.
    pub(crate) fn path(&self) -> SitePath {
        SitePath::new(format!(
            "/blog/{}/{}/{}/{}",
            self.year.of(POSTS_BLOB),
            self.month.of(POSTS_BLOB),
            self.date.of(POSTS_BLOB),
            self.slug.of(POSTS_BLOB)
        ))
    }

    /// Where the post lives on the web — the external host, or this site.
    pub(crate) fn url(&self) -> String {
        match self.external_url {
            Some(url) => url.of(POSTS_BLOB).decrypt(),
            None => format!("https://developersam.com{}", self.path()),
        }
    }
}

/// The post a site path names, if it is one this site hosts.
pub(crate) fn find(path: &SitePath) -> Option<usize> {
    POSTS
        .iter()
        .position(|post| !post.is_external() && &post.path() == path)
}
