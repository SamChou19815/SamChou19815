import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";
import { BLOG_TITLE } from "../../lib/blog-constants";

export const metadata: Metadata = {
  title: BLOG_TITLE,
};

export default function BlogListPage(): React.JSX.Element {
  return <TerminalPage />;
}
