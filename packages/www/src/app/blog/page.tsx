import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";
import { BLOG_TITLE } from "../../lib/blog-constants";

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

export default function BlogListPage(): React.JSX.Element {
  return <TerminalPage />;
}
