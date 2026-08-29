// @ts-check

const fs = require("node:fs");
const path = require("node:path");

/**
 * The post's frontmatter: a `---` fenced block of `key: "value"` lines at the
 * very top of the file. Only `title` and the optional hero `image` live there,
 * so a hand-rolled reader is enough and the site keeps no YAML dependency.
 * `crates/sam-tui/build.rs` parses the same block for the TUI.
 */
function parseFrontmatter(/** @type {string} */ source, /** @type {string} */ fullPath) {
  const lines = source.split("\n");
  if (lines[0] !== "---") {
    throw new Error(`${fullPath}: expected a --- frontmatter fence on the first line.`);
  }
  /** @type {Record<string, string>} */
  const fields = {};
  for (let index = 1; index < lines.length; index++) {
    const line = lines[index];
    if (line === "---") {
      if (fields.title == null) {
        throw new Error(`${fullPath}: the frontmatter has no title.`);
      }
      return fields;
    }
    const match = /^([a-z]+): "(.*)"$/.exec(line ?? "");
    if (match == null) {
      throw new Error(`${fullPath}: invalid frontmatter line:\n${line}`);
    }
    fields[match[1]] = match[2];
  }
  throw new Error(`${fullPath}: the frontmatter fence is never closed.`);
}

function computeAllMedatada() {
  const BLOG_POSTS_ROOT = path.join("src", "blog-posts");

  /** @type {import("./src/lib/metadata").BlogPostMetadata[]} */
  const allMetadata = [];

  /** @type {Omit<import("./src/lib/metadata").BlogPostMetadata, 'titleSlug'>[]} */
  const externalPosts = JSON.parse(
    fs.readFileSync(path.join("src", "external-blog-posts.json")).toString(),
  );
  for (const post of externalPosts) {
    if (post.externalUrl == null) {
      throw new Error(`External blog post "${post.title}" is missing an externalUrl.`);
    }
    allMetadata.push({ titleSlug: "", ...post });
  }

  for (const year of fs.readdirSync(BLOG_POSTS_ROOT)) {
    for (const month of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year))) {
      for (const date of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year, month))) {
        for (const file of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year, month, date))) {
          const fullPath = path.join(BLOG_POSTS_ROOT, year, month, date, file);
          const { title, image } = parseFrontmatter(fs.readFileSync(fullPath).toString(), fullPath);
          allMetadata.push({
            title,
            year,
            month,
            date,
            titleSlug: path.basename(file, ".md"),
            ...(image == null ? {} : { image }),
          });
        }
      }
    }
  }

  allMetadata.sort((a, b) => {
    let c = b.year.localeCompare(a.year);
    if (c !== 0) return c;
    c = b.month.localeCompare(a.month);
    if (c !== 0) return c;
    c = b.date.localeCompare(a.date);
    return c;
  });

  return allMetadata;
}

/** @type {import('next').NextConfig} */
const nextConfig = {
  experimental: { useTypeScriptCli: true },
  output: "export",
  env: { ALL_BLOG_POST_METADATA: JSON.stringify(computeAllMedatada()) },
  typescript: { ignoreBuildErrors: true },
};

module.exports = nextConfig;
