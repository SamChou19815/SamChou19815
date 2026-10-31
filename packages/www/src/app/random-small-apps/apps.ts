// The registry of the small apps under /random-small-apps. Each app keeps its
// whole state as one JSON document per user in the shared `random_small_apps`
// table (see smallAppStorage.ts), so registering an app is just naming it.
export type RandomSmallApp = {
  // The app's id, also its key in the shared table.
  id: string;
  name: string;
  // One-line summary shown on the launcher card.
  tagline: string;
  // The app's route path.
  path: string;
};

export const RANDOM_SMALL_APPS: readonly RandomSmallApp[] = [
  {
    id: "wwm-gear-tracker",
    name: "WWM Gear Tracker",
    tagline:
      "Define categories of affixes and how many of each your whole Where Winds Meet gear set needs, then track the shared graduation rate.",
    path: "/random-small-apps/wwm-gear-tracker",
  },
  {
    id: "edit-json",
    name: "JSON Editor",
    tagline: "Manually edit any small app's saved JSON document.",
    path: "/random-small-apps/edit-json",
  },
];
