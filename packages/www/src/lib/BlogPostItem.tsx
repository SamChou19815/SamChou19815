export default function BlogPostItem({
  title,
  formattedDate,
  children,
}: {
  title: React.JSX.Element | string;
  formattedDate: string;
  children?: React.ReactNode;
}): React.JSX.Element {
  const TitleHeading = typeof title === "string" ? "h1" : "h2";

  return (
    <article className="mb-4 rounded-md border border-solid border-gray-200 bg-white p-4 font-serif drop-shadow-sm filter transition-all duration-300 hover:drop-shadow-md dark:border-gray-700 dark:bg-[#242424] dark:drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)] dark:hover:drop-shadow-[0_4px_8px_rgba(0,0,0,0.4)]">
      <header>
        <TitleHeading className="mb-2 font-sans">{title}</TitleHeading>
        <time className="text-sm text-gray-500 dark:text-gray-400">{formattedDate}</time>
      </header>
      {children}
    </article>
  );
}
