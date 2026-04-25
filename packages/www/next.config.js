// @ts-check

const fs = require("node:fs");
const path = require("node:path");

function parseMarkdownTitle(/** @type {string} */ source) {
  const firstLine = source.split("\n")[0];
  if (firstLine == null) {
    throw new Error("No title.");
  }
  const START = 'export const title = "';
  if (!firstLine.startsWith(START) || !firstLine.endsWith('";')) {
    throw new Error(`Invalid title line:\n${firstLine}`);
  }
  return firstLine.substring(START.length, firstLine.length - 2).trim();
}

function computeAllMedatada() {
  const BLOG_POSTS_ROOT = path.join("src", "app", "blog", "(blog-posts)");

  /** @type {import("./src/lib/metadata").BlogPostMetadata[]} */
  const allMetadata = [];
  for (const year of fs.readdirSync(BLOG_POSTS_ROOT)) {
    if (year === "layout.tsx") {
      continue;
    }
    for (const month of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year))) {
      for (const date of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year, month))) {
        for (const titleSlug of fs.readdirSync(path.join(BLOG_POSTS_ROOT, year, month, date))) {
          const fullPath = path.join(BLOG_POSTS_ROOT, year, month, date, titleSlug, "page.mdx");
          const content = fs.readFileSync(fullPath).toString();
          try {
            const title = parseMarkdownTitle(content);
            allMetadata.push({ title, year, month, date, titleSlug });
          } catch (error) {
            throw new Error(`Failed to parse ${fullPath}`, { cause: error });
          }
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

const withMDX = require("@next/mdx")({
  extension: /\.mdx?$/,
  options: {
    remarkPlugins: [],
    rehypePlugins: [],
  },
});

/** @type {import('next').NextConfig} */
const nextConfig = {
  pageExtensions: ["tsx", "mdx"],
  experimental: { mdxRs: true },
  output: "export",
  env: { ALL_BLOG_POST_METADATA: JSON.stringify(computeAllMedatada()) },
  typescript: { ignoreBuildErrors: true },
};

module.exports = withMDX(nextConfig);
