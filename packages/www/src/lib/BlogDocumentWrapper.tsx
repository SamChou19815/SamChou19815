import type { ReactNode } from "react";
import NavBar from "./NavBar";
import { BLOG_TITLE } from "./blog-constants";

export default function BlogDocumentWrapper({
  children,
}: {
  children: ReactNode;
}): React.JSX.Element {
  return (
    <>
      <NavBar title={BLOG_TITLE} titleLink="/blog" navItems={[{ name: "Home", link: "/" }]} />
      <div className="lg:max-w-screen-lg my-8 mx-auto">{children}</div>
    </>
  );
}
