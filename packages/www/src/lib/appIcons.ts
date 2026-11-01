// Per-page icons, so each personal tool can be added to a phone's home screen
// as its own app. Every icon lives in `public/app-icons` as `<id>.svg` (the
// source, also the browser tab icon), `<id>-{180,192,512}.png` rendered from
// it, and `<id>.manifest.json`, whose name, start URL and colors are the app's.
//
// TanStack's HeadContent merges `meta` by name but keeps every route's `links`,
// so the icon and manifest links are not in the root route: each page sets
// exactly one set, either an app's or the site's.

export type AppIconId =
  | "wwm-gear-tracker"
  | "edit-json"
  | "random-small-apps"
  | "budget"
  | "in-canada";

type HeadTags = {
  meta: { name: string; content: string }[];
  links: { rel: string; href: string; type?: string; sizes?: string }[];
};

/**
 * The icon and manifest tags for one app.
 *
 * @param shortName The label under the home-screen icon; keep it short.
 */
export function appIconHead(id: AppIconId, shortName: string): HeadTags {
  return {
    meta: [
      { name: "apple-mobile-web-app-title", content: shortName },
      // Opens the home-screen icon without browser chrome on older iOS, which
      // ignores the manifest's `display`.
      { name: "apple-mobile-web-app-capable", content: "yes" },
    ],
    links: [
      { rel: "icon", type: "image/svg+xml", href: `/app-icons/${id}.svg` },
      { rel: "apple-touch-icon", sizes: "180x180", href: `/app-icons/${id}-180.png` },
      { rel: "manifest", href: `/app-icons/${id}.manifest.json` },
    ],
  };
}

/** The site's own icon and manifest, for every page that is not an app. */
export const SITE_ICON_LINKS: HeadTags["links"] = [
  { rel: "manifest", href: "/manifest.json" },
  { rel: "icon", href: "/logo.png" },
  { rel: "shortcut icon", href: "/favicon.ico" },
];
