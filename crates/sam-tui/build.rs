//! Bakes the homepage's card artwork into the binary as small RGB pixel grids,
//! and compiles the blog's posts and image dimensions into `posts.rs`.
//!
//! The TUI paints images as half-blocks (`▀`), so one cell carries two stacked
//! pixels and the pixel grid is roughly square. Each asset is fitted into
//! [`MAX_COLS`]x[`MAX_ROWS`] cells — the largest any placement asks for — and
//! resampled once, here, at build time. The runtime box-filters that grid down
//! for smaller placements, so a single bake serves both the card thumbnails and
//! the modal hero.
//!
//! Baking pixels rather than finished `(fg, bg)` cell pairs costs the same six
//! bytes per cell and stays resamplable. The whole set is ~63KB.
//!
//! Blog images take the other road: only their *dimensions* are recorded
//! (`len == 0` in the index), so the binary does not grow by a single pixel.
//! The TUI reserves the cell box and draws a captioned frame there; the web
//! overlay then lays the real, full-resolution file over the reported region.
//!
//! The blog's post list is compiled here too — titles, dates, thumbnails,
//! excerpts and full bodies — mirroring `computeAllMedatda()` in
//! `packages/www/next.config.js`, so the TUI and the site can never disagree
//! about what the blog contains.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The largest cell box any placement uses (the detail modal's hero).
const MAX_COLS: u32 = 56;
const MAX_ROWS: u32 = 16;

/// Site-root-relative asset paths, which double as the `<img src>` the web
/// overlay needs and as the key `data.rs` refers to them by.
const ASSETS: &[&str] = &[
    "/timeline/canada.webp",
    "/timeline/courseplan-promotion.png",
    "/timeline/critter-compiler.webp",
    "/timeline/critter-world.webp",
    "/timeline/fb-sticker-pen.webp",
    "/timeline/flow.webp",
    "/timeline/pyre-check.webp",
    "/timeline/sam-later.webp",
    "/timeline/sampl.webp",
    "/timeline/website-v2.webp",
    "/timeline/website-v3.webp",
    "/projects/mini-react.webp",
    "/projects/samlang.webp",
    "/projects/samwise.webp",
    "/projects/ten.webp",
    "/fan-arts/graduation-sam.webp",
    "/sam-by-megan-3-square.webp",
];

/// Cells an image of `width`x`height` occupies when fitted into the box,
/// preserving aspect. Two pixels per cell vertically is what makes the
/// comparison against a 1:1 pixel grid correct.
fn fit(width: u32, height: u32) -> (u32, u32) {
    let mut cols = MAX_COLS;
    let mut rows = ((cols as f64 * height as f64) / (2.0 * width as f64)).round() as u32;
    if rows > MAX_ROWS {
        rows = MAX_ROWS;
        cols = ((2.0 * rows as f64 * width as f64) / height as f64).round() as u32;
    }
    (cols.clamp(1, MAX_COLS), rows.max(1))
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let public = manifest.join("../../packages/www/public");
    let www_src = manifest.join("../../packages/www/src");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    println!("cargo:rerun-if-changed=build.rs");

    let mut blob: Vec<u8> = Vec::new();
    let mut index = String::new();

    for url in ASSETS {
        let source: &Path = &public.join(url.trim_start_matches('/'));
        println!("cargo:rerun-if-changed={}", source.display());

        let decoded = image::ImageReader::open(source)
            .unwrap_or_else(|error| panic!("opening {}: {error}", source.display()))
            .decode()
            .unwrap_or_else(|error| panic!("decoding {}: {error}", source.display()))
            .to_rgb8();
        let (cols, rows) = fit(decoded.width(), decoded.height());
        let resized = image::imageops::resize(
            &decoded,
            cols,
            rows * 2,
            image::imageops::FilterType::Lanczos3,
        );

        let offset = blob.len();
        blob.extend_from_slice(resized.as_raw());
        writeln!(
            index,
            "    Baked {{ url: {url:?}, cols: {cols}, rows: {rows}, offset: {offset}, len: {} }},",
            blob.len() - offset,
        )
        .expect("writing to a String cannot fail");
    }

    // Blog images: dimensions only, so the binary stays blind to their pixels.
    // The `len: 0` sentinel tells the runtime to draw a placeholder frame and
    // report the region for the web overlay, exactly as a baked image does.
    for url in blog_image_urls(&public) {
        let source = public.join(url.trim_start_matches('/'));
        println!("cargo:rerun-if-changed={}", source.display());
        let (width, height) = image::ImageReader::open(&source)
            .unwrap_or_else(|error| panic!("opening {}: {error}", source.display()))
            .into_dimensions()
            .unwrap_or_else(|error| panic!("reading {}'s dimensions: {error}", source.display()));
        let (cols, rows) = fit(width, height);
        writeln!(
            index,
            "    Baked {{ url: {url:?}, cols: {cols}, rows: {rows}, offset: 0, len: 0 }},"
        )
        .expect("writing to a String cannot fail");
    }

    std::fs::write(out.join("images.bin"), &blob).expect("writing images.bin");
    std::fs::write(
        out.join("images.rs"),
        format!(
            "// @generated by build.rs — do not edit.\n\
             static BLOB: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/images.bin\"));\n\
             static INDEX: &[Baked] = &[\n{index}];\n"
        ),
    )
    .expect("writing images.rs");

    std::fs::write(out.join("posts.rs"), compile_posts(&public, &www_src))
        .expect("writing posts.rs");
}

/// Site URLs of every image under `public/blog`, sorted so the generated index
/// is byte-stable across machines.
fn blog_image_urls(public: &Path) -> Vec<String> {
    fn walk(dir: &Path, public: &Path, urls: &mut Vec<String>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("reading {}: {error}", dir.display()))
            .map(|entry| entry.expect("reading a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, public, urls);
            } else {
                let relative = path
                    .strip_prefix(public)
                    .expect("the walk starts under public");
                urls.push(format!(
                    "/{}",
                    relative.to_string_lossy().replace('\\', "/")
                ));
            }
        }
    }
    let mut urls = Vec::new();
    walk(&public.join("blog"), public, &mut urls);
    urls
}

// --- Compiling the blog --------------------------------------------------------

/// One post between reading its sources and writing `posts.rs`.
struct PostSource {
    title: String,
    year: String,
    month: String,
    date: String,
    /// Empty for external posts, which have no page on this site.
    slug: String,
    external_url: Option<String>,
    thumbnail: Option<String>,
    excerpt: String,
    /// Markdown body with the MDX header stripped. Empty for external posts.
    body: String,
}

fn compile_posts(public: &Path, www_src: &Path) -> String {
    let mut posts = local_posts(public, www_src);
    posts.extend(external_posts(www_src));
    // Newest first, compared as strings — the components are zero-padded, so
    // this matches next.config.js's localeCompare sort exactly.
    posts.sort_by(|a, b| {
        b.year
            .cmp(&a.year)
            .then_with(|| b.month.cmp(&a.month))
            .then_with(|| b.date.cmp(&a.date))
    });

    let mut code =
        String::from("// @generated by build.rs — do not edit.\npub static POSTS: &[Post] = &[\n");
    for post in &posts {
        write!(
            code,
            "    Post {{\n\
             \x20       title: {:?},\n\
             \x20       year: {:?}, month: {:?}, date: {:?},\n\
             \x20       slug: {:?},\n\
             \x20       external_url: {},\n\
             \x20       thumbnail: {},\n\
             \x20       excerpt: {:?},\n\
             \x20       body: {:?},\n\
             \x20   }},\n",
            post.title,
            post.year,
            post.month,
            post.date,
            post.slug,
            optional_str(&post.external_url),
            optional_str(&post.thumbnail),
            post.excerpt,
            post.body,
        )
        .expect("writing to a String cannot fail");
    }
    code.push_str("];\n");
    code
}

/// A generated `Option<&str>` literal: `None` or `Some("…")`.
fn optional_str(value: &Option<String>) -> String {
    value
        .as_deref()
        .map_or_else(|| "None".to_string(), |url| format!("Some({url:?})"))
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("reading {}: {error}", dir.display()))
        .map(|entry| entry.expect("reading a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    entries
}

fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("reading {}: {error}", dir.display()))
        .map(|entry| entry.expect("reading a directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    entries.sort();
    entries
}

/// Walks `blog-posts/<year>/<month>/<date>/<slug>.md`, as
/// `computeAllMedatada()` does. The slug is the file stem, so the permalink a
/// post is served at is spelled out by its path and nothing else.
fn local_posts(public: &Path, www_src: &Path) -> Vec<PostSource> {
    let root = www_src.join("blog-posts");
    let mut posts = Vec::new();
    for year in sorted_dirs(&root) {
        for month in sorted_dirs(&year) {
            for date in sorted_dirs(&month) {
                for page in sorted_files(&date) {
                    println!("cargo:rerun-if-changed={}", page.display());
                    let source = std::fs::read_to_string(&page)
                        .unwrap_or_else(|error| panic!("reading {}: {error}", page.display()));
                    posts.push(parse_post(
                        &source,
                        &page,
                        public,
                        &year
                            .file_name()
                            .expect("a year dir is named")
                            .to_string_lossy(),
                        &month
                            .file_name()
                            .expect("a month dir is named")
                            .to_string_lossy(),
                        &date
                            .file_name()
                            .expect("a date dir is named")
                            .to_string_lossy(),
                        &page
                            .file_stem()
                            .expect("a post file is named")
                            .to_string_lossy(),
                    ));
                }
            }
        }
    }
    posts
}

fn parse_post(
    source: &str,
    page: &Path,
    public: &Path,
    year: &str,
    month: &str,
    date: &str,
    slug: &str,
) -> PostSource {
    // A `---` fenced block of `key: "value"` lines opens every post, and the
    // body is everything after it — the same rule next.config.js panics on.
    let (front, body) = frontmatter(source, page);
    let title = front
        .iter()
        .find(|(key, _)| *key == "title")
        .unwrap_or_else(|| panic!("{}: the frontmatter has no title", page.display()))
        .1
        .clone();

    // The hero image is the optional `image` field, which is a site path.
    let hero = front
        .iter()
        .find(|(key, _)| *key == "image")
        .and_then(|(_, url)| resolve_url(url, public));
    let thumbnail = hero.or_else(|| {
        body_images(&body)
            .iter()
            .find_map(|url| resolve_url(url, public))
    });
    // The stored body's image URLs are site-root-relative, so the runtime can
    // borrow each one straight out of it. Prose links keep their absolute
    // form: the native binary opens them with `open`, which needs the host.
    let body = normalize_body_image_urls(&body);

    PostSource {
        title,
        year: year.to_string(),
        month: month.to_string(),
        date: date.to_string(),
        slug: slug.to_string(),
        external_url: None,
        thumbnail,
        excerpt: excerpt(&body),
        body,
    }
}

/// Rewrites the blog's fully-qualified image URLs to site-root-relative ones,
/// outside code fences. Markdown links on other lines keep their host.
fn normalize_body_image_urls(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_code = false;
    for line in body.lines() {
        let mut replaced = line.to_string();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
        } else if !in_code {
            replaced = replaced.replace(r#"src="https://developersam.com/"#, r#"src=""#);
            if trimmed.starts_with("![") {
                replaced = replaced.replace("](https://developersam.com/", "](/");
            }
        }
        out.push_str(&replaced);
        out.push('\n');
    }
    out
}

/// Splits a post into its frontmatter fields and its body. The fence must open
/// the file, every field is `key: "value"`, and the body starts on the line
/// after the closing fence.
fn frontmatter(source: &str, page: &Path) -> (Vec<(String, String)>, String) {
    let mut lines = source.lines();
    if lines.next() != Some("---") {
        panic!(
            "{}: expected a --- frontmatter fence on the first line",
            page.display()
        );
    }
    let mut fields = Vec::new();
    for line in lines.by_ref() {
        if line == "---" {
            let mut body = String::new();
            for line in lines.skip_while(|line| line.trim().is_empty()) {
                body.push_str(line);
                body.push('\n');
            }
            return (fields, body);
        }
        let (key, value) = line
            .split_once(": ")
            .and_then(|(key, value)| Some((key, value.strip_prefix('"')?.strip_suffix('"')?)))
            .unwrap_or_else(|| panic!("{}: invalid frontmatter line: {line}", page.display()));
        fields.push((key.to_string(), value.to_string()));
    }
    panic!("{}: the frontmatter fence is never closed", page.display());
}

/// A `![alt](url)` reference on a line, if the line carries one.
fn markdown_image(line: &str) -> Option<(&str, &str)> {
    let start = line.find("![")? + 2;
    let rest = &line[start..];
    let close = rest.find("](")?;
    let after = &rest[close + 2..];
    let end = after.find(')')?;
    Some((&rest[..close], &after[..end]))
}

/// Markdown image URLs in a body, outside code fences — `vec![` inside a
/// ```rust``` block is Rust, not an image.
fn body_images(body: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut in_code = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        } else if !in_code {
            if let Some((_, url)) = markdown_image(line) {
                urls.push(url.to_string());
            }
        }
    }
    urls
}

/// Normalizes `https://developersam.com/X` to `/X` and keeps the URL only if
/// the site really serves it — a dead legacy path must not become an image the
/// web would 404 on.
fn resolve_url(url: &str, public: &Path) -> Option<String> {
    let path = url
        .strip_prefix("https://developersam.com/")
        .map_or(url.to_string(), |rest| format!("/{rest}"));
    if !path.starts_with('/') || !public.join(path.trim_start_matches('/')).exists() {
        return None;
    }
    Some(path)
}

/// Whether a trimmed line opens a numbered list item, like `3. `.
fn numbered_item(line: &str) -> bool {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && line[digits..].starts_with(". ")
}

/// The first paragraph of a body: the first run of consecutive non-blank lines
/// that is not a heading, fence, image, quote, HTML or list item, joined with
/// single spaces, stripped of inline markdown and cut at 200 chars.
fn excerpt(body: &str) -> String {
    let mut in_code = false;
    let mut paragraph: Vec<&str> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let structural = trimmed.starts_with('#')
            || trimmed.starts_with('!')
            || trimmed.starts_with('>')
            || trimmed.starts_with('<')
            || trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || numbered_item(trimmed);
        if trimmed.starts_with("```") {
            in_code = !in_code;
        }
        if trimmed.is_empty() || in_code || structural || trimmed.starts_with("```") {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed);
    }
    let text = strip_inline(&paragraph.join(" "));
    text.chars().take(200).collect()
}

/// Renders inline markdown as the plain text it reads as: emphasis markers
/// dropped, links collapsed to their text.
fn strip_inline(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        plain.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        if let Some(close) = after.find("](") {
            if let Some(end) = after[close + 2..].find(')').map(|index| close + 2 + index) {
                plain.push_str(&after[..close]);
                rest = &after[end + 1..];
                continue;
            }
        }
        plain.push('[');
        rest = after;
    }
    plain.push_str(rest);
    plain.replace("**", "").replace(['`', '*'], "")
}

/// The posts that live elsewhere, from `external-blog-posts.json`.
fn external_posts(www_src: &Path) -> Vec<PostSource> {
    let file = www_src.join("external-blog-posts.json");
    println!("cargo:rerun-if-changed={}", file.display());
    let json = std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("reading {}: {error}", file.display()));
    let entries: Vec<serde_json::Value> = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", file.display()));
    let field = |entry: &serde_json::Value, name: &str| {
        entry[name]
            .as_str()
            .unwrap_or_else(|| panic!("{name} is missing in {}", file.display()))
            .to_string()
    };
    entries
        .iter()
        .map(|entry| PostSource {
            title: field(entry, "title"),
            year: field(entry, "year"),
            month: field(entry, "month"),
            date: field(entry, "date"),
            slug: String::new(),
            external_url: Some(field(entry, "externalUrl")),
            thumbnail: None,
            excerpt: String::new(),
            body: String::new(),
        })
        .collect()
}
