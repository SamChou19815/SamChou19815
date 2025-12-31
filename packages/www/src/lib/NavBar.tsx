import Link from "next/link";

export default function NavBar({
  title,
  titleLink,
  navItems,
}: {
  title: string;
  titleLink: string;
  navItems: ReadonlyArray<{ readonly name: string; readonly link: string }>;
}): React.JSX.Element {
  return (
    <nav className="sticky top-0 z-40 flex h-16 bg-white pr-4 drop-shadow-sm filter">
      <div className="flex w-full flex-wrap justify-between">
        <div className="flex min-w-0 flex-auto items-center">
          <Link
            className="mr-8 flex min-w-0 items-center text-gray-900 transition-opacity duration-200 hover:opacity-80"
            href={titleLink}
          >
            <img
              className="mr-4 h-16 flex-initial"
              src="/sam-by-megan-3-square.webp"
              alt="dev-sam fan art"
            />
            <strong className="flex-auto text-lg font-semibold">{title}</strong>
          </Link>
        </div>
        <div className="flex min-w-0 flex-initial items-center justify-end">
          {navItems.map(({ name, link }) => (
            <Link
              key={link}
              className="rounded px-3 py-1 font-medium text-gray-900 transition-colors duration-200 hover:bg-gray-100 hover:text-blue-500"
              href={link}
            >
              {name}
            </Link>
          ))}
        </div>
      </div>
    </nav>
  );
}
