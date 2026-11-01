import { createFileRoute, Link } from "@tanstack/react-router";
import { appIconHead } from "../../lib/appIcons";
import { RANDOM_SMALL_APPS } from "../../app/random-small-apps/apps";

export const Route = createFileRoute("/random-small-apps/")({
  head: () => {
    const icon = appIconHead("random-small-apps", "Small Apps");
    return {
      meta: [
        { title: "Random Small Apps | Developer Sam" },
        { name: "robots", content: "noindex, nofollow" },
        ...icon.meta,
      ],
      links: icon.links,
    };
  },
  component: RandomSmallAppsIndexPage,
});

function RandomSmallAppsIndexPage(): React.JSX.Element {
  return (
    <div className="mx-auto w-full max-w-6xl px-6 py-12">
      <h1>Random Small Apps</h1>
      <p className="mb-8 max-w-2xl text-sm text-gray-500 dark:text-gray-400">
        Small personal tools. Each one keeps its whole state as a single JSON document per account —
        sign in from the app itself to load yours.
      </p>
      <div className="grid gap-4 sm:grid-cols-2">
        {RANDOM_SMALL_APPS.map((app) => (
          <Link
            key={app.id}
            to={app.path}
            className="flex items-start gap-4 rounded bg-white p-6 filter drop-shadow transition-all duration-300 ease-out hover:drop-shadow-lg dark:bg-[#242424] dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]"
          >
            <img
              src={`/app-icons/${app.id}.svg`}
              alt=""
              width={56}
              height={56}
              className="size-14 shrink-0 rounded-xl"
            />
            <span className="flex min-w-0 flex-col">
              <span className="text-lg font-semibold">{app.name}</span>
              <span className="mt-2 text-sm text-gray-500 dark:text-gray-400">{app.tagline}</span>
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}
