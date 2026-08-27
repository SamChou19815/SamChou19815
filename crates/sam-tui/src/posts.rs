//! The blog index and post bodies, generated from packages/www by build.rs.

pub struct Post {
    pub title: &'static str,
    pub year: &'static str,
    pub month: &'static str,
    pub date: &'static str,
    /// Empty for external posts, which have no page on this site.
    pub slug: &'static str,
    pub external_url: Option<&'static str>,
    /// Card artwork: the post's OG image, else its first inline image.
    pub thumbnail: Option<&'static str>,
    pub excerpt: &'static str,
    /// Markdown body with the MDX header stripped. Empty for external posts.
    pub body: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/posts.rs"));

impl Post {
    pub fn is_external(&self) -> bool {
        self.external_url.is_some()
    }

    pub fn formatted_date(&self) -> String {
        format!("{}-{}-{}", self.year, self.month, self.date)
    }

    /// Where the post lives on the web — the external host, or this site.
    pub fn url(&self) -> String {
        match self.external_url {
            Some(url) => url.to_string(),
            None => format!(
                "https://developersam.com/blog/{}/{}/{}/{}",
                self.year, self.month, self.date, self.slug
            ),
        }
    }
}
