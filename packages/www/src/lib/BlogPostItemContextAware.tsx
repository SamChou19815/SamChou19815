"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import BlogPostItem from "./BlogPostItem";
import { allMetadata, isExternalPost, permalinkFromMetadata } from "./metadata";

function PaginationNavItem({
  permalink,
  title,
  isLeft,
  isExternal,
}: {
  permalink: string;
  title: string;
  isLeft: boolean;
  isExternal: boolean;
}): React.JSX.Element {
  const className =
    "flex-grow rounded-md border border-solid border-gray-300 p-4 leading-tight hover:border-blue-500 dark:border-gray-600 dark:hover:border-blue-400";
  const titleText = isLeft ? `« ${title}` : `${title} »`;
  const inner = (
    <>
      <div className="mb-1 text-sm font-medium text-gray-500 dark:text-gray-400">
        {isLeft ? "Newer Post" : "Older Post"}
      </div>
      <div className="break-words font-bold">{isExternal ? `${titleText} ↗` : titleText}</div>
    </>
  );
  return isExternal ? (
    <a className={className} href={permalink} target="_blank" rel="noopener noreferrer">
      {inner}
    </a>
  ) : (
    <Link className={className} href={permalink}>
      {inner}
    </Link>
  );
}

export default function BlogPostItemContextAware({ children }: { children: React.ReactNode }) {
  const currentPath = usePathname();
  const index = allMetadata.findIndex((it) => currentPath.startsWith(permalinkFromMetadata(it)));
  const metadata = allMetadata[index];
  if (metadata == null) throw currentPath;
  const prevMetadata = allMetadata[index - 1];
  const nextMetadata = allMetadata[index + 1];

  return (
    <div className="flex flex-row flex-wrap justify-center">
      <main className="w-full">
        <BlogPostItem
          title={metadata.title}
          formattedDate={`${metadata.year}-${metadata.month}-${metadata.date}`}
        >
          <div className="markdown">{children}</div>
        </BlogPostItem>
        <div className="my-8">
          <nav className="flex" aria-label="Blog post page navigation">
            <div className="flex flex-1">
              {prevMetadata && (
                <PaginationNavItem
                  permalink={permalinkFromMetadata(prevMetadata)}
                  title={prevMetadata.title}
                  isLeft={true}
                  isExternal={isExternalPost(prevMetadata)}
                />
              )}
            </div>
            <div className="ml-4 flex flex-1 text-right">
              {nextMetadata && (
                <PaginationNavItem
                  permalink={permalinkFromMetadata(nextMetadata)}
                  title={nextMetadata.title}
                  isLeft={false}
                  isExternal={isExternalPost(nextMetadata)}
                />
              )}
            </div>
          </nav>
        </div>
      </main>
    </div>
  );
}
