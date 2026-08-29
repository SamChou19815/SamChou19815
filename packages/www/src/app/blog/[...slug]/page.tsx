import type { Metadata } from "next";
import TerminalPage from "../../../lib/TerminalPage";
import { BLOG_TITLE } from "../../../lib/blog-constants";
import { allMetadata, isExternalPost, permalinkFromMetadata } from "../../../lib/metadata";

type Params = { readonly slug: readonly string[] };

const localPosts = allMetadata.filter((post) => !isExternalPost(post));

// One exported shell per permalink, so a post's URL is a real document that
// loads the terminal and opens the reader on it. The prose itself is compiled
// into the wasm binary from `src/blog-posts` by `crates/sam-tui/build.rs`.
export function generateStaticParams(): Params[] {
  return localPosts.map(({ year, month, date, titleSlug }) => ({
    slug: [year, month, date, titleSlug],
  }));
}

export async function generateMetadata({ params }: { params: Promise<Params> }): Promise<Metadata> {
  const { slug } = await params;
  const permalink = `/blog/${slug.join("/")}`;
  const post = localPosts.find((it) => permalinkFromMetadata(it) === permalink);
  if (post == null) return {};
  const title = `${post.title} | ${BLOG_TITLE}`;
  return {
    title,
    openGraph: { type: "article", title, images: post.image },
  };
}

export default function BlogPostPage(): React.JSX.Element {
  return <TerminalPage />;
}
