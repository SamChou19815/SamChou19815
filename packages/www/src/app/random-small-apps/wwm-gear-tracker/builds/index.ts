// Every build calculator of wherewindsmeetcalculator.com, ported. Each port is
// checked against the site's own code to floating-point precision.

import { BAMBOOCUT_DUST } from "./bamboocut-dust";
import { BAMBOOCUT_KITE } from "./bamboocut-kite";
import { BAMBOOCUT_WIND } from "./bamboocut-wind";
import { BELLSTRIKE_SPLENDOR } from "./bellstrike-splendor";
import { BELLSTRIKE_UMBRA } from "./bellstrike-umbra";
import { eraseInput, type AnyBuild } from "./common";
import { SILKBIND_JADE } from "./silkbind-jade";
import { STONESPLIT_MIGHT } from "./stonesplit-might";
import { STONESPLIT_SCALE } from "./stonesplit-scale";

export type { AnyBuild } from "./common";

/** In the site's order: 鸣金, 牵丝, 破竹, 裂石. */
export const BUILDS: readonly AnyBuild[] = [
  eraseInput(BELLSTRIKE_UMBRA),
  eraseInput(BELLSTRIKE_SPLENDOR),
  eraseInput(SILKBIND_JADE),
  eraseInput(BAMBOOCUT_DUST),
  eraseInput(BAMBOOCUT_KITE),
  eraseInput(BAMBOOCUT_WIND),
  eraseInput(STONESPLIT_MIGHT),
  eraseInput(STONESPLIT_SCALE),
];

export const DEFAULT_BUILD_ID = STONESPLIT_SCALE.id;

export function buildById(id: string): AnyBuild {
  const build =
    BUILDS.find((candidate) => candidate.id === id) ??
    BUILDS.find((candidate) => candidate.id === DEFAULT_BUILD_ID);
  if (build == null) throw new Error("No builds registered");
  return build;
}
