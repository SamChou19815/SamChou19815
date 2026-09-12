//! Compiles the blog's posts into `posts.rs`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The cipher the crate itself uses, included rather than duplicated — a build
/// script cannot depend on the crate it builds. Only the encrypting half is
/// wanted here; the crate uses the rest.
#[allow(dead_code)]
mod cipher {
    include!("src/cipher.rs");
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let www_src = manifest.join("../../packages/www/src");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cipher.rs");

    let (posts, posts_blob) = compile_posts(&www_src);
    std::fs::write(out.join("posts.bin"), &posts_blob).expect("writing posts.bin");
    std::fs::write(out.join("posts.rs"), posts).expect("writing posts.rs");
}

// --- Compiling the blog --------------------------------------------------------

/// The blog's name.
const BLOG_TITLE: &str = "Developer Sam Blog";

/// One post between reading its sources and writing `posts.rs`.
struct PostSource {
    title: String,
    year: String,
    month: String,
    date: String,
    /// Empty for external posts, which have no page on this site.
    slug: String,
    external_url: Option<String>,
    /// Markdown body with the MDX header stripped. Empty for external posts.
    body: String,
}

fn compile_posts(www_src: &Path) -> (String, Vec<u8>) {
    let mut posts = local_posts(www_src);
    posts.extend(external_posts(www_src));
    posts.sort_by(|a, b| {
        b.year
            .cmp(&a.year)
            .then_with(|| b.month.cmp(&a.month))
            .then_with(|| b.date.cmp(&a.date))
    });

    let mut blog_post_encrypted_blob: Vec<u8> = Vec::new();
    let mut code = format!(
        "// @{} by build.rs — do not edit.\n\
         static POSTS_BLOB: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/posts.bin\"));\n\
         /// The blog's name.\n\
         const BLOG_TITLE: EncryptedRun = {};\n\
         pub static POSTS: &[Post] = &[\n",
        "generated",
        encrypted(&mut blog_post_encrypted_blob, BLOG_TITLE)
    );
    for post in &posts {
        write!(
            code,
            "    Post {{\n\
             \x20       title: {},\n\
             \x20       year: {}, month: {}, date: {},\n\
             \x20       slug: {},\n\
             \x20       external_url: {},\n\
             \x20       body: {},\n\
             \x20   }},\n",
            encrypted(&mut blog_post_encrypted_blob, &post.title),
            encrypted(&mut blog_post_encrypted_blob, &post.year),
            encrypted(&mut blog_post_encrypted_blob, &post.month),
            encrypted(&mut blog_post_encrypted_blob, &post.date),
            encrypted(&mut blog_post_encrypted_blob, &post.slug),
            optional_encrypted(&mut blog_post_encrypted_blob, &post.external_url),
            encrypted(&mut blog_post_encrypted_blob, &post.body),
        )
        .expect("writing to a String cannot fail");
    }
    code.push_str("];\n");
    (code, blog_post_encrypted_blob)
}

fn encrypted(blog_post_encrypted_blob: &mut Vec<u8>, text: &str) -> String {
    let seed = cipher::seed_of(text);
    let start = blog_post_encrypted_blob.len();
    blog_post_encrypted_blob.extend(
        text.bytes()
            .enumerate()
            .map(|(index, byte)| cipher::scramble(byte, seed, index)),
    );
    format!("EncryptedRun::new({seed}, {start}, {})", text.len())
}

fn optional_encrypted(blog_post_encrypted_blob: &mut Vec<u8>, value: &Option<String>) -> String {
    value.as_deref().map_or_else(
        || "None".to_string(),
        |text| format!("Some({})", encrypted(blog_post_encrypted_blob, text)),
    )
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

fn local_posts(www_src: &Path) -> Vec<PostSource> {
    let root = www_src.join("blog-posts");
    let mut posts = Vec::new();
    for year in sorted_dirs(&root) {
        for month in sorted_dirs(&year) {
            for date in sorted_dirs(&month) {
                for page_md_path in sorted_files(&date) {
                    println!("cargo:rerun-if-changed={}", page_md_path.display());
                    let source = std::fs::read_to_string(&page_md_path).unwrap_or_else(|error| {
                        panic!("reading {}: {error}", page_md_path.display())
                    });
                    posts.push(parse_post(
                        &source,
                        &page_md_path,
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
                        &page_md_path
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
    page_md_path: &Path,
    year: &str,
    month: &str,
    date: &str,
    slug: &str,
) -> PostSource {
    let (front, body) = frontmatter(source, page_md_path);
    let title = front
        .iter()
        .find(|(key, _)| *key == "title")
        .unwrap_or_else(|| panic!("{}: the frontmatter has no title", page_md_path.display()))
        .1
        .clone();
    PostSource {
        title,
        year: year.to_string(),
        month: month.to_string(),
        date: date.to_string(),
        slug: slug.to_string(),
        external_url: None,
        body,
    }
}

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
            body: String::new(),
        })
        .collect()
}
