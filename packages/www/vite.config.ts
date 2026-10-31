import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import viteReact from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { computeAllMetadata } from "./blog-metadata.ts";

const allMetadata = computeAllMetadata();

// One prerendered shell per permalink, so a post's URL is a real document that
// loads the terminal and opens the reader on it. The prose itself is compiled
// into the wasm binary from `src/blog-posts` by `crates/sam-web/build.rs`.
const blogPostPaths = allMetadata
  .filter((post) => post.externalUrl == null)
  .map(({ year, month, date, titleSlug }) => `/blog/${year}/${month}/${date}/${titleSlug}`);

const pagePaths = [
  "/",
  "/about",
  "/blog",
  "/timeline",
  "/budget",
  "/in-canada",
  "/random-small-apps",
  "/random-small-apps/wwm-gear-tracker",
  "/random-small-apps/edit-json",
  ...blogPostPaths,
];

export default defineConfig({
  define: {
    "process.env.ALL_BLOG_POST_METADATA": JSON.stringify(JSON.stringify(allMetadata)),
  },
  plugins: [
    tanstackStart({
      // The site is deployed as static files: every page is rendered at build
      // time into dist/client. `autoSubfolderIndex: false` writes `/about` as
      // `about.html` rather than `about/index.html`, so URLs keep no trailing
      // slash. The wasm app renders no links for the crawler to follow.
      prerender: { enabled: true, autoSubfolderIndex: false, crawlLinks: false },
      pages: pagePaths.map((path) => ({ path })),
    }),
    viteReact(),
    tailwindcss(),
  ],
});
