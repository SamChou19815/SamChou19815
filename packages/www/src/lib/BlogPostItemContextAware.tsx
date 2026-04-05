"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import BlogPostItem from "./BlogPostItem";
import { allMetadata, permalinkFromMetadata } from "./metadata";

function PaginationNavItem({
  permalink,
  title,
  isLeft,
}: {
  permalink: string;
  title: string;
  isLeft: boolean;
}): React.JSX.Element {
  return (
    <Link
      className="flex-grow rounded-md border border-solid border-gray-300 p-4 leading-tight hover:border-blue-500 dark:border-gray-600 dark:hover:border-blue-400"
      href={permalink}
    >
      <div className="mb-1 text-sm font-medium text-gray-500 dark:text-gray-400">
        {isLeft ? "Newer Post" : "Older Post"}
      </div>
      <div className="break-words font-bold">{isLeft ? `« ${title}` : `${title} »`}</div>
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
                />
              )}
            </div>
            <div className="ml-4 flex flex-1 text-right">
              {nextMetadata && (
                <PaginationNavItem
                  permalink={permalinkFromMetadata(nextMetadata)}
                  title={nextMetadata.title}
                  isLeft={false}
                />
              )}
            </div>
          </nav>
        </div>
      </main>
    </div>
  );
}
