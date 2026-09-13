//! Compiles the blog's posts into `posts.rs`, their fenced code blocks
//! highlighted by tree-sitter grammars.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use tree_sitter::Language;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

/// The cipher the crate itself uses, included rather than duplicated — a build
/// script cannot depend on the crate it builds. Only the encrypting half is
/// wanted here; the crate uses the rest.
#[allow(dead_code)]
mod cipher {
    include!("src/crypt/cipher.rs");
}

/// The fence scanner the renderer uses, included the same way, so the blocks
/// highlighted here are exactly the blocks the renderer draws.
#[allow(dead_code)]
mod markdown_scan {
    include!("src/markdown/scan.rs");
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let www_src = manifest.join("../../packages/www/src");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/crypt/cipher.rs");
    println!("cargo:rerun-if-changed=src/markdown/scan.rs");

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
    /// One entry per fenced block in the body, in order: its highlighting.
    code_blocks: Vec<CompiledBlock>,
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
    highlight_posts(&mut posts);

    let mut blog_post_encrypted_blob: Vec<u8> = Vec::new();
    let mut code = format!(
        "// @{} by build.rs — do not edit.\n\
         static POSTS_BLOB: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/posts.bin\"));\n\
         /// The blog's name.\n\
         const BLOG_TITLE: EncryptedRun = {};\n\
         pub(crate) static POSTS: &[Post] = &[\n",
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
             \x20       code_blocks: {},\n\
             \x20   }},\n",
            encrypted(&mut blog_post_encrypted_blob, &post.title),
            encrypted(&mut blog_post_encrypted_blob, &post.year),
            encrypted(&mut blog_post_encrypted_blob, &post.month),
            encrypted(&mut blog_post_encrypted_blob, &post.date),
            encrypted(&mut blog_post_encrypted_blob, &post.slug),
            optional_encrypted(&mut blog_post_encrypted_blob, &post.external_url),
            encrypted(&mut blog_post_encrypted_blob, &post.body),
            code_blocks_expr(&post.code_blocks),
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
        code_blocks: Vec::new(),
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
            code_blocks: Vec::new(),
        })
        .collect()
}

// --- Tree-sitter highlighting --------------------------------------------------
//
// The grammars run here, on the host, at build time: the highlighting joins
// the post it belongs to as static data, byte ranges and palette colors with
// no text of their own, and the wasm binary carries no grammar at all.

/// A recognized tree-sitter capture name and the palette color it resolves
/// to, in the renderer's `SpanColor` terms. `None` is a deliberate plain:
/// recognizing the name lets it outmatch a shorter name that would color.
const CAPTURE_COLORS: &[(&str, Option<&str>)] = &[
    // Escapes before constants: a `constant.character.escape` capture holds
    // both names, and the tie is broken by order.
    ("escape", Some("Keyword")),
    ("comment", Some("Comment")),
    ("string.special.key", Some("Function")),
    ("string.special", Some("String")),
    ("string.regexp", None),
    ("string", Some("String")),
    ("constant.numeric", Some("Number")),
    ("number", Some("Number")),
    ("constant.builtin", Some("Keyword")),
    ("boolean", Some("Keyword")),
    ("constant", Some("Type")),
    ("function.builtin", Some("Keyword")),
    ("function", Some("Function")),
    ("method", Some("Function")),
    ("type.builtin", Some("Keyword")),
    ("type", Some("Type")),
    ("constructor", Some("Type")),
    ("tag", Some("Keyword")),
    ("attribute", Some("Type")),
    ("variable.builtin", Some("Keyword")),
    ("label", Some("Keyword")),
    ("keyword.operator", None),
    ("keyword", Some("Keyword")),
];

/// One grammar the highlighter can be pointed at.
struct Grammar {
    /// The name injections call this language by.
    name: &'static str,
    language: Language,
    highlights: String,
    injections: &'static str,
    locals: String,
}

fn grammar(
    name: &'static str,
    language: Language,
    highlights: String,
    injections: &'static str,
    locals: String,
) -> Grammar {
    Grammar {
        name,
        language,
        highlights,
        injections,
        locals,
    }
}

/// Query strings joined into one, the way a grammar's own query files would
/// be if it shipped them whole.
fn joined(queries: &[&str]) -> String {
    queries.join("\n")
}

fn grammars() -> Vec<Grammar> {
    use tree_sitter_bash as bash;
    use tree_sitter_c as c;
    use tree_sitter_c_sharp as c_sharp;
    use tree_sitter_cpp as cpp;
    use tree_sitter_css as css;
    use tree_sitter_go as go;
    use tree_sitter_html as html;
    use tree_sitter_java as java;
    use tree_sitter_javascript as javascript;
    use tree_sitter_json as json;
    use tree_sitter_php as php;
    use tree_sitter_python as python;
    use tree_sitter_ruby as ruby;
    use tree_sitter_rust as rust;
    use tree_sitter_swift as swift;
    use tree_sitter_typescript as typescript;
    use tree_sitter_yaml as yaml;

    // The typescript crate ships its highlight query as a supplement to the
    // javascript one — the grammars share their node names, so the two join
    // into the whole query every editor composes for the language.
    let typescript_highlights =
        joined(&[javascript::HIGHLIGHT_QUERY, typescript::HIGHLIGHTS_QUERY]);
    let tsx_highlights = joined(&[
        javascript::HIGHLIGHT_QUERY,
        javascript::JSX_HIGHLIGHT_QUERY,
        typescript::HIGHLIGHTS_QUERY,
    ]);
    let typescript_locals = joined(&[javascript::LOCALS_QUERY, typescript::LOCALS_QUERY]);

    vec![
        grammar(
            "typescript",
            Language::new(typescript::LANGUAGE_TYPESCRIPT),
            typescript_highlights,
            "",
            typescript_locals.clone(),
        ),
        grammar(
            "tsx",
            Language::new(typescript::LANGUAGE_TSX),
            tsx_highlights,
            "",
            typescript_locals,
        ),
        grammar(
            "javascript",
            Language::new(javascript::LANGUAGE),
            joined(&[javascript::HIGHLIGHT_QUERY, javascript::JSX_HIGHLIGHT_QUERY]),
            javascript::INJECTIONS_QUERY,
            javascript::LOCALS_QUERY.to_string(),
        ),
        grammar(
            "rust",
            Language::new(rust::LANGUAGE),
            rust::HIGHLIGHTS_QUERY.to_string(),
            rust::INJECTIONS_QUERY,
            String::new(),
        ),
        grammar(
            "json",
            Language::new(json::LANGUAGE),
            // The grammar paints a key like any other string; a later pattern
            // for the same node wins, so this one gives keys their own color
            // back.
            joined(&[
                json::HIGHLIGHTS_QUERY,
                "(pair key: (string) @string.special.key)",
            ]),
            "",
            String::new(),
        ),
        grammar(
            "bash",
            Language::new(bash::LANGUAGE),
            bash::HIGHLIGHT_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "python",
            Language::new(python::LANGUAGE),
            python::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "java",
            Language::new(java::LANGUAGE),
            java::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "go",
            Language::new(go::LANGUAGE),
            go::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "c",
            Language::new(c::LANGUAGE),
            c::HIGHLIGHT_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "cpp",
            Language::new(cpp::LANGUAGE),
            cpp::HIGHLIGHT_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "c-sharp",
            Language::new(c_sharp::LANGUAGE),
            c_sharp::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "css",
            Language::new(css::LANGUAGE),
            css::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "html",
            Language::new(html::LANGUAGE),
            html::HIGHLIGHTS_QUERY.to_string(),
            html::INJECTIONS_QUERY,
            String::new(),
        ),
        grammar(
            "yaml",
            Language::new(yaml::LANGUAGE),
            yaml::HIGHLIGHTS_QUERY.to_string(),
            "",
            String::new(),
        ),
        grammar(
            "swift",
            Language::new(swift::LANGUAGE),
            swift::HIGHLIGHTS_QUERY.to_string(),
            swift::INJECTIONS_QUERY,
            swift::LOCALS_QUERY.to_string(),
        ),
        grammar(
            "php",
            Language::new(php::LANGUAGE_PHP),
            php::HIGHLIGHTS_QUERY.to_string(),
            php::INJECTIONS_QUERY,
            String::new(),
        ),
        grammar(
            "ruby",
            Language::new(ruby::LANGUAGE),
            ruby::HIGHLIGHTS_QUERY.to_string(),
            "",
            ruby::LOCALS_QUERY.to_string(),
        ),
    ]
}

/// The language table the configs are built in: canonical names, which are
/// also what injections call them by, and the extra fence labels that reach
/// each one.
const LANGUAGES: &[(&str, &[&str])] = &[
    ("typescript", &["ts"]),
    ("tsx", &[]),
    ("javascript", &["js", "jsx"]),
    ("rust", &["rs"]),
    ("json", &[]),
    ("bash", &["sh", "shell", "zsh"]),
    ("python", &["py"]),
    ("java", &[]),
    ("go", &["golang"]),
    ("c", &[]),
    ("cpp", &["c++", "cxx"]),
    ("c-sharp", &["csharp", "c#", "cs"]),
    ("css", &[]),
    ("html", &[]),
    ("yaml", &["yml"]),
    ("swift", &[]),
    ("php", &[]),
    ("ruby", &["rb"]),
];

/// The compiled highlighting of one fenced block: per line, its spans.
struct CompiledBlock {
    /// Per line: where in the line each span starts, how long it is, and its
    /// palette color. Empty when the block gets no highlighting.
    lines: Vec<Vec<(u32, u32, &'static str)>>,
}

/// Highlights every post's fenced blocks, in place.
fn highlight_posts(posts: &mut [PostSource]) {
    let mut highlighter = Highlighter::new();
    let names: Vec<&str> = CAPTURE_COLORS.iter().map(|(name, _)| *name).collect();
    let configs: Vec<(&str, HighlightConfiguration)> = grammars()
        .into_iter()
        .map(|it| {
            let mut config = HighlightConfiguration::new(
                it.language,
                it.name,
                &it.highlights,
                it.injections,
                &it.locals,
            )
            .unwrap_or_else(|error| panic!("compiling the {} grammar: {error}", it.name));
            config.configure(&names);
            (it.name, config)
        })
        .collect();
    for post in posts {
        if post.body.is_empty() {
            continue;
        }
        post.code_blocks = body_code_blocks(&mut highlighter, &configs, &post.body);
    }
}

/// One fenced block while scanning a body: its label and raw lines.
struct ScannedBlock<'a> {
    lang: &'a str,
    lines: Vec<&'a str>,
}

/// All of a body's fenced blocks, scanned the way the renderer scans them.
fn body_blocks(body: &str) -> Vec<ScannedBlock<'_>> {
    use markdown_scan::{scan_line, LineKind};
    let mut blocks = Vec::new();
    let mut open: Option<ScannedBlock> = None;
    let mut in_code = false;
    for line in body.lines() {
        match scan_line(line, &mut in_code) {
            LineKind::FenceOpen(lang) => {
                open = Some(ScannedBlock {
                    lang,
                    lines: Vec::new(),
                });
            }
            LineKind::FenceClose => {
                if let Some(block) = open.take() {
                    blocks.push(block);
                }
            }
            LineKind::Code(source) => {
                if let Some(block) = open.as_mut() {
                    block.lines.push(source);
                }
            }
            LineKind::Text(_) => {}
        }
    }
    // A block left open at the end of the body is still drawn — highlighted,
    // then, like any other.
    blocks.extend(open);
    blocks
}

fn body_code_blocks(
    highlighter: &mut Highlighter,
    configs: &[(&str, HighlightConfiguration)],
    body: &str,
) -> Vec<CompiledBlock> {
    body_blocks(body)
        .into_iter()
        .map(|block| {
            // The first word of the label names the language; anything after
            // it is metadata to whatever wrote the label.
            let label = block.lang.split_whitespace().next().unwrap_or("");
            let config = configs.iter().find(|(name, _)| {
                label.eq_ignore_ascii_case(name)
                    || LANGUAGES.iter().any(|(language, aliases)| {
                        language == name
                            && aliases
                                .iter()
                                .any(|alias| label.eq_ignore_ascii_case(alias))
                    })
            });
            match config {
                // Samlang keeps the runtime highlighter; text and unknown
                // labels are drawn plain.
                None => CompiledBlock { lines: Vec::new() },
                Some((_, config)) => CompiledBlock {
                    lines: highlight_block(highlighter, configs, config, &block.lines),
                },
            }
        })
        .collect()
}

/// The config table entry a language name reaches, by the name injections
/// call it by.
fn config_by_name<'a>(
    configs: &'a [(&str, HighlightConfiguration)],
    name: &str,
) -> Option<&'a HighlightConfiguration> {
    configs
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, config)| config)
}

fn highlight_block(
    highlighter: &mut Highlighter,
    configs: &[(&str, HighlightConfiguration)],
    config: &HighlightConfiguration,
    lines: &[&str],
) -> Vec<Vec<(u32, u32, &'static str)>> {
    let source = lines.join("\n");
    // Injections name their language the way the config table does.
    let mut injected = |name: &str| config_by_name(configs, name);
    let Ok(events) = highlighter.highlight(config, source.as_bytes(), None, None, &mut injected)
    else {
        return vec![Vec::new(); lines.len()];
    };
    // The innermost highlight covering a source range paints it; the ranges
    // tile the source, so the colored ones flatten into one list.
    let mut colored: Vec<(usize, usize, &'static str)> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for event in events.flatten() {
        match event {
            HighlightEvent::HighlightStart(Highlight(index)) => stack.push(index),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                if let Some(color) = stack.last().and_then(|&index| CAPTURE_COLORS[index].1) {
                    colored.push((start, end, color));
                }
            }
        }
    }
    // Split the colored ranges into the lines the renderer will paint.
    let mut per_line = vec![Vec::new(); lines.len()];
    let mut line_start = 0;
    for (line_index, line) in lines.iter().enumerate() {
        let line_end = line_start + line.len();
        for &(start, end, color) in &colored {
            let start = start.max(line_start);
            let end = end.min(line_end);
            if start < end {
                per_line[line_index].push((
                    (start - line_start) as u32,
                    (end - start) as u32,
                    color,
                ));
            }
        }
        line_start = line_end + 1;
    }
    per_line
}

/// The `code_blocks` expression for one post's compiled highlighting.
fn code_blocks_expr(blocks: &[CompiledBlock]) -> String {
    let mut out = String::from("&[");
    for block in blocks {
        out.push_str("CodeBlock { lines: &[");
        for line in &block.lines {
            out.push_str("&[");
            for (start, len, color) in line {
                write!(
                    out,
                    "CodeSpan {{ start: {start}, len: {len}, color: SpanColor::{color} }}, "
                )
                .expect("writing to a String cannot fail");
            }
            out.push_str("], ");
        }
        out.push_str("]}, ");
    }
    out.push(']');
    out
}
