// What changing 词条 is worth in DPS: the tracker's worn set priced with the
// chosen build's model. Every figure is relative to the panel the user entered,
// which is taken to be their panel with the current worn set on.

import {
  affixHasEffect,
  currentBuild,
  dpsChangePercent,
  evaluate,
  maxRoll,
  rolledAmount,
  settingsInput,
  swapAffixes,
  type DpsResult,
  type PricedAffix,
} from "./dpsModel";
import {
  AFFIXES,
  equippedGears,
  wornBySlot,
  type Affix,
  type Gear,
  type GearSlot,
  type RolledAffix,
  type WwmGearTrackerData,
} from "./types";

/** The 词条 the chosen build's model gives a damage value. */
export function pricedAffixList(data: WwmGearTrackerData): Affix[] {
  const build = currentBuild(data.dps);
  return AFFIXES.filter((affix) => affixHasEffect(build, affix));
}

/** Whether the chosen build's model prices `affix`. */
export function isPriced(data: WwmGearTrackerData, affix: Affix): boolean {
  return affixHasEffect(currentBuild(data.dps), affix);
}

/** The current panel's DPS. */
export function currentDps(data: WwmGearTrackerData): DpsResult {
  return evaluate(currentBuild(data.dps), settingsInput(data.dps), data.dps.level);
}

/** A piece's rolled 词条 with their amounts; blank values count as the best roll. */
export function pricedAffixes(
  affixes: readonly Pick<RolledAffix, "affix" | "value">[],
  level: number,
): PricedAffix[] {
  return affixes.map(({ affix, value }) => ({ affix, amount: rolledAmount(affix, value, level) }));
}

/** DPS before and after a change, and the relative change. */
export type DpsChange = {
  before: DpsResult;
  after: DpsResult;
  // Relative DPS change, as a percentage.
  percent: number;
};

/** The build, input and current DPS every change in one analysis is priced against. */
type Pricing = {
  data: WwmGearTrackerData;
  build: ReturnType<typeof currentBuild>;
  input: object;
  before: DpsResult;
};

function pricing(data: WwmGearTrackerData): Pricing {
  const build = currentBuild(data.dps);
  const input = settingsInput(data.dps);
  return { data, build, input, before: evaluate(build, input, data.dps.level) };
}

function change(
  { data, build, input, before }: Pricing,
  removed: PricedAffix[],
  added: PricedAffix[],
): DpsChange {
  const after = evaluate(build, swapAffixes(build, input, removed, added), data.dps.level);
  return { before, after, percent: dpsChangePercent(before, after) };
}

/** Wearing a piece with `affixes` in place of the worn pieces `replacing`. */
export function wearChange(
  data: WwmGearTrackerData,
  affixes: readonly Pick<RolledAffix, "affix" | "value">[],
  replacing: readonly Gear[],
): DpsChange {
  const { level } = data.dps;
  return change(
    pricing(data),
    replacing.flatMap((gear) => pricedAffixes(gear.affixes, level)),
    pricedAffixes(affixes, level),
  );
}

/** The worn piece in `slot`, as a list to pass to `wearChange`. */
export function wornIn(data: WwmGearTrackerData, slot: GearSlot): Gear[] {
  const worn = wornBySlot(data).get(slot);
  return worn == null ? [] : [worn];
}

/** Adding one best-rolled 词条 to the worn set. */
export type AffixMarginal = { affix: Affix; amount: number; change: DpsChange };

/** Each priced 词条 at its best roll, added on top of the current panel, best first. */
export function affixMarginals(data: WwmGearTrackerData): AffixMarginal[] {
  const context = pricing(data);
  return pricedAffixList(data)
    .map((affix) => {
      const amount = maxRoll(affix, data.dps.level);
      return { affix, amount, change: change(context, [], [{ affix, amount }]) };
    })
    .sort((a, b) => b.change.percent - a.change.percent);
}

/** One worn 词条: what it is worth, and the best 词条 to reroll it into. */
export type WornAffixPayoff = {
  gear: Gear;
  rolled: RolledAffix;
  amount: number;
  // Losing the 词条 outright: `percent` is negative for a 词条 that helps.
  removal: DpsChange;
  // The best other 词条 at its best roll in its place, or null when none is priced.
  best: { affix: Affix; amount: number; change: DpsChange } | null;
};

/** Every worn 词条 priced, the most worth rerolling first. */
export function wornAffixPayoffs(data: WwmGearTrackerData): WornAffixPayoff[] {
  const { level } = data.dps;
  const context = pricing(data);
  const candidates = pricedAffixList(data);
  const payoffs: WornAffixPayoff[] = [];
  for (const gear of equippedGears(data)) {
    for (const rolled of gear.affixes) {
      const amount = rolledAmount(rolled.affix, rolled.value, level);
      const removed = [{ affix: rolled.affix, amount }];
      let best: WornAffixPayoff["best"] = null;
      for (const affix of candidates) {
        if (affix === rolled.affix) continue;
        const candidate = { affix, amount: maxRoll(affix, level) };
        const swapped = change(context, removed, [candidate]);
        if (best == null || swapped.percent > best.change.percent) {
          best = { ...candidate, change: swapped };
        }
      }
      payoffs.push({ gear, rolled, amount, removal: change(context, removed, []), best });
    }
  }
  return payoffs.sort(
    (a, b) => (b.best?.change.percent ?? -Infinity) - (a.best?.change.percent ?? -Infinity),
  );
}

/** A signed percentage like "+1.23%" or "−0.40%". */
export function formatChange(percent: number, digits = 2): string {
  const rounded = Number(percent.toFixed(digits));
  if (rounded === 0) return `±${(0).toFixed(digits)}%`;
  return `${rounded > 0 ? "+" : "−"}${Math.abs(rounded).toFixed(digits)}%`;
}

/** A DPS figure like "65,011". */
export function formatDps(dps: number): string {
  return Math.round(dps).toLocaleString("en-US");
}
