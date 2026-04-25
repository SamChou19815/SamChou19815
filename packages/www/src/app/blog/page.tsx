import type { Metadata } from "next";
import Link from "next/link";
import BlogDocumentWrapper from "../../lib/BlogDocumentWrapper";
import BlogPostItem from "../../lib/BlogPostItem";
import { BLOG_TITLE } from "../../lib/blog-constants";
import { allMetadata, permalinkFromMetadata } from "../../lib/metadata";

export const metadata: Metadata = {
  title: BLOG_TITLE,
  description: "Developer Sam's Blog",
  authors: { name: "Developer Sam" },
  openGraph: {
    images: "https://developersam.com/sam-by-megan-3-square.webp",
    type: "profile",
    title: BLOG_TITLE,
    description: "Developer Sam's Blog",
  },
};

export default async function BlogListPage(): Promise<React.JSX.Element> {
  return (
    <BlogDocumentWrapper>
      <div className="flex flex-row flex-wrap justify-center">
        <main className="w-full">
          {allMetadata.map((post) => {
            const permalink = permalinkFromMetadata(post);
            return (
              <BlogPostItem
                key={permalink}
                title={<Link href={permalink}>{post.title}</Link>}
                formattedDate={`${post.year}-${post.month}-${post.date}`}
              />
            );
          })}
        </main>
      </div>
    </BlogDocumentWrapper>
  );
}
