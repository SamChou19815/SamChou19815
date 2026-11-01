import { createRootRoute, HeadContent, Outlet, Scripts } from "@tanstack/react-router";
import commonCss from "../lib/common.css?url";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { name: "author", content: "Developer Sam" },
    ],
    links: [
      { rel: "stylesheet", href: commonCss },
      { rel: "canonical", href: "https://developersam.com" },
      // Icons and the manifest are set per page: see `lib/appIcons.ts`.
    ],
  }),
  component: RootLayout,
  notFoundComponent: NotFound,
});

function RootLayout(): React.JSX.Element {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        <Outlet />
        <Scripts />
      </body>
    </html>
  );
}

// Static hosting serves `public/404.html` for unknown URLs; this only shows on
// a client-side navigation to one.
function NotFound(): React.JSX.Element {
  return (
    <div className="flex h-screen flex-col items-center justify-center text-center">
      <h1 className="text-2xl font-medium">404: This page could not be found.</h1>
    </div>
  );
}
