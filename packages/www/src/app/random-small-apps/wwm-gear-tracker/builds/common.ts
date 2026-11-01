// Pieces the per-build DPS formulas share. Each formula is a port of one build
// calculator on wherewindsmeetcalculator.com; they agree on how a hit resolves
// but each tunes its own constants, so only the plumbing lives here.

import { TARGETS } from "./shared.data";

export const DEFAULT_TARGET = "100定极分更";

/** The training target's defense and stat modifiers, falling back to 100定极分更. */
export function targetOf(name: string): (typeof TARGETS)[string] {
  const target = TARGETS[name] ?? TARGETS[DEFAULT_TARGET];
  if (target == null) throw new Error(`Unknown target ${name}`);
  return target;
}

/** How likely a hit is to graze, crit, land as affinity, or land plainly. */
export type Chances = { glance: number; crit: number; aff: number; plain: number };

/**
 * Outcome chances from final crit and affinity chances and precision. Affinity
 * hits never graze; `neverGlances` makes the rest never graze either.
 */
export function chances(
  critChance: number,
  affChance: number,
  precision: number,
  neverGlances = false,
): Chances {
  const glance = (1 - (neverGlances ? 1 : precision)) * (1 - affChance);
  const crit = critChance + affChance <= 1 ? precision * critChance : precision * (1 - affChance);
  return { glance, crit, aff: affChance, plain: Math.max(1 - glance - crit - affChance, 0) };
}

/**
 * Expected damage of a hit that deals `min` on a graze, `avg` on a crit or a
 * plain hit and `max` on an affinity hit, each times `scale` and its outcome's
 * multiplier.
 */
export function expectedHit(
  p: Chances,
  min: number,
  avg: number,
  max: number,
  scale: number,
  multipliers: { glance: number; crit: number; aff: number },
): number {
  return (
    min * scale * multipliers.glance * p.glance +
    avg * scale * multipliers.crit * p.crit +
    max * scale * multipliers.aff * p.aff +
    avg * scale * p.plain
  );
}

/** One 属攻's panel stats. */
export type ElementStats = { min: number; max: number; penetration: number; damageBonus: number };

export const ELEMENTS = ["鸣金", "裂石", "牵丝", "破竹"] as const;

export type Element = (typeof ELEMENTS)[number];

/** The panel key prefix of each 属攻. */
export const ELEMENT_KEYS: Readonly<Record<Element, string>> = {
  鸣金: "bellstrike",
  裂石: "stonesplit",
  牵丝: "qiansi",
  破竹: "pozhu",
};

/** A 属攻's stats off a panel; zeros for anything else (or a field the panel lacks). */
export function elementOf(panel: object, element: string): ElementStats {
  const key = (ELEMENT_KEYS as Record<string, string | undefined>)[element];
  if (key == null) return { min: 0, max: 0, penetration: 0, damageBonus: 0 };
  const read = (field: string) => (panel as Record<string, number | undefined>)[key + field] ?? 0;
  return {
    min: read("AttackMin"),
    max: read("AttackMax"),
    penetration: read("Penetration"),
    damageBonus: read("DamageBonus"),
  };
}

/** The site's flags are sometimes 1 and sometimes "1". */
export function isOne(value: unknown): boolean {
  return value === 1 || value === "1";
}

/** What every build's formula exposes to the tracker. */
export type BuildDefinition<Input> = {
  id: string;
  // The build's in-game name, e.g. 裂石钧.
  name: string;
  // Key into BASELINE_DPS.
  baselineKey: string;
  preset: Input;
  // The rotation's DPS for `input`.
  dps: (input: Input) => number;
  // The weaponBonus key 武学增效 counts toward: the build's main weapon.
  mainWeapon: string;
  // Whether the panel keeps 精准/会心/会意 as raw in-game percentages (the formula
  // applies judgment resistance itself) rather than as final fractions.
  rawRates: boolean;
};

/** A build with its input type erased, so builds can sit in one list. */
export type AnyBuild = Omit<BuildDefinition<object>, "dps"> & { dps: (input: object) => number };

export function eraseInput<Input extends object>(build: BuildDefinition<Input>): AnyBuild {
  // Inputs only ever come from this build's preset, with numbers changed.
  return { ...build, dps: (input) => build.dps(input as Input) };
}
