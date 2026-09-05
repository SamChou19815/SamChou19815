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

pub struct Post {
    title: EncryptedRun,
    year: EncryptedRun,
    month: EncryptedRun,
    date: EncryptedRun,
    /// Empty for external posts, which have no page on this site.
    slug: EncryptedRun,
    external_url: Option<EncryptedRun>,
    /// Card artwork: the post's OG image, else its first inline image.
    thumbnail: Option<EncryptedRun>,
    excerpt: EncryptedRun,
    /// Markdown body with the frontmatter stripped. Empty for external posts.
    body: EncryptedRun,
}

include!(concat!(env!("OUT_DIR"), "/posts.rs"));

/// The blog's name, read from `blog-constants.ts` by build.rs so the TUI and
/// the site can never disagree about what to call it.
pub fn blog_title() -> EncryptedString {
    BLOG_TITLE.of(POSTS_BLOB)
}

impl Post {
    pub fn title(&self) -> EncryptedString {
        self.title.of(POSTS_BLOB)
    }

    pub fn excerpt(&self) -> EncryptedString {
        self.excerpt.of(POSTS_BLOB)
    }

    /// Markdown body with the frontmatter stripped. Empty for external posts.
    pub fn body(&self) -> EncryptedString {
        self.body.of(POSTS_BLOB)
    }

    /// Card artwork: the post's OG image, else its first inline image.
    pub fn thumbnail(&self) -> Option<EncryptedString> {
        self.thumbnail.map(|run| run.of(POSTS_BLOB))
    }

    pub fn is_external(&self) -> bool {
        self.external_url.is_some()
    }

    pub fn formatted_date(&self) -> String {
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
    pub fn path(&self) -> String {
        format!(
            "/blog/{}/{}/{}/{}",
            self.year.of(POSTS_BLOB),
            self.month.of(POSTS_BLOB),
            self.date.of(POSTS_BLOB),
            self.slug.of(POSTS_BLOB)
        )
    }

    /// Where the post lives on the web — the external host, or this site.
    pub fn url(&self) -> String {
        match self.external_url {
            Some(url) => url.of(POSTS_BLOB).decrypt(),
            None => format!("https://developersam.com{}", self.path()),
        }
    }
}

/// The post a site path names, if it is one this site hosts.
pub fn find(path: &str) -> Option<usize> {
    POSTS
        .iter()
        .position(|post| !post.is_external() && post.path() == path)
}
