// Prices 词条 in DPS with the build calculators of wherewindsmeetcalculator.com
// (ported in ./builds). A build's input is its calculator's preset with the
// numbers the user entered laid over it; a 词条 change moves those numbers.

import { buildById, BUILDS, DEFAULT_BUILD_ID, type AnyBuild } from "./builds";
import { ELEMENT_KEYS, targetOf } from "./builds/common";
import { BASELINE_DPS, CONVERSION_RATES, MAX_ROLLS } from "./builds/shared.data";
import type { Affix } from "./types";

/** Entered values by field path, e.g. "panelAttributes.criticalRate". */
export type PanelOverrides = Record<string, number>;

/** The DPS model's saved settings. */
export type DpsSettings = {
  build: string;
  level: number;
  // Per build, so switching builds keeps each one's panel.
  panels: Record<string, PanelOverrides>;
};

/** Character levels the site has a 毕业 baseline for. */
export const LEVELS: readonly number[] = [
  ...new Set(Object.values(BASELINE_DPS).flatMap((byLevel) => Object.keys(byLevel).map(Number))),
].sort((a, b) => a - b);

export const DEFAULT_LEVEL = 100;

export function defaultDpsSettings(): DpsSettings {
  return { build: DEFAULT_BUILD_ID, level: DEFAULT_LEVEL, panels: {} };
}

// ---- Fields ----------------------------------------------------------------

/** A number in a build's input the user can enter. */
export type PanelField = {
  path: string;
  group: string;
  label: string;
  // Entered and shown ×100 (stored as a fraction).
  percent: boolean;
  // Shown with a % sign but stored as entered (raw in-game percentages).
  rawPercent: boolean;
};

const GROUP_LABELS: Readonly<Record<string, string>> = {
  panelAttributes: "面板",
  weaponBonus: "武学增效",
  skillBonus: "奇术增伤",
  generalBonus: "通用增伤",
  tuningBonus: "定音",
  buffs: "增益",
  rangeBonus: "距离增伤",
  xiaoyinBonus: "箫音增伤",
  "": "其他",
};

// Groups whose numbers are not panel stats.
const SKIPPED_GROUPS = new Set(["martialArts"]);

const FIELD_LABELS: Readonly<Record<string, string>> = {
  physicalAttackMin: "最小外功攻击",
  physicalAttackMax: "最大外功攻击",
  physicalPenetration: "外功穿透",
  physicalDamageBonus: "外功增伤",
  wuxiangAttackMin: "最小无相攻击",
  wuxiangAttackMax: "最大无相攻击",
  wuxiangPenetration: "无相穿透",
  precisionRate: "精准率",
  criticalRate: "会心率",
  affinityRate: "会意率",
  directCriticalRate: "直接会心率",
  directAffinityRate: "直接会意率",
  scratchDamageBonus: "擦伤伤害加成",
  critDamageBonus: "会心伤害加成",
  affDamageBonus: "会意伤害加成",
  dotDamageBonus: "持续伤害加成",
  fixedDamageBonus: "固定伤害加成",
  "weaponBonus.sword": "剑武学增效",
  "weaponBonus.spear": "枪武学增效",
  "weaponBonus.qiang": "枪武学增效",
  "weaponBonus.umbrella": "伞武学增效",
  "weaponBonus.fan": "扇武学增效",
  "weaponBonus.ropeDart": "绳镖武学增效",
  "weaponBonus.fist": "手甲武学增效",
  "weaponBonus.dualBlade": "双刀武学增效",
  "weaponBonus.modao": "陌刀武学增效",
  "weaponBonus.hengdao": "横刀武学增效",
  "weaponBonus.allWeapon": "全武学增效",
  "weaponBonus.boss": "首领增伤",
  "weaponBonus.na": "无武器技能增伤",
  "weaponBonus.umbrellaQDingyin": "伞 Q 定音",
  "skillBonus.singleTarget": "单体奇术增伤",
  "skillBonus.aoe": "群体奇术增伤",
  "generalBonus.general": "通用增伤",
  "generalBonus.common": "通用增伤",
  "generalBonus.charge": "蓄力增伤",
  "generalBonus.dingyin": "定音增伤",
  "generalBonus.fistCharge": "手甲蓄力定音",
  "tuningBonus.swordQ": "剑 Q 定音",
  "tuningBonus.swordWave": "剑气定音",
  "tuningBonus.swordR": "剑 R 定音",
  "tuningBonus.swordCharge": "剑蓄力定音",
  "tuningBonus.bleed": "流血定音",
  "tuningBonus.spearQ": "枪 Q 定音",
  "tuningBonus.spearR": "枪 R 定音",
  "tuningBonus.umbrellaCharge": "伞蓄力定音",
  "rangeBonus.closeRange": "近战增伤",
  "rangeBonus.longRange": "远程增伤",
  "xiaoyinBonus.melee": "近战箫音",
  "xiaoyinBonus.ranged": "远程箫音",
  "buffs.tieyishuhun": "铁衣束魂",
  fistExpectation: "手甲期望系数",
  jiuqiangLayers: "九枪层数",
};

const ELEMENT_NAMES: Readonly<Record<string, string>> = Object.fromEntries(
  Object.entries(ELEMENT_KEYS).map(([name, key]) => [key, name]),
);

function fieldLabel(path: string): string {
  const key = path.split(".").at(-1) ?? path;
  const known = FIELD_LABELS[path] ?? FIELD_LABELS[key];
  if (known != null) return known;
  const element =
    /^(bellstrike|stonesplit|qiansi|pozhu)(AttackMin|AttackMax|Penetration|DamageBonus)$/.exec(key);
  if (element != null) {
    const name = ELEMENT_NAMES[element[1] ?? ""] ?? "";
    return (
      {
        AttackMin: `最小${name}攻击`,
        AttackMax: `最大${name}攻击`,
        Penetration: `${name}穿透`,
        DamageBonus: `${name}增伤`,
      }[element[2] ?? ""] ?? key
    );
  }
  return key;
}

// Panel stats that are plain numbers rather than shares.
const PLAIN_KEY = /Attack(Min|Max)$|Penetration$/;
const RAW_RATE_KEYS = new Set([
  "precisionRate",
  "criticalRate",
  "affinityRate",
  "directCriticalRate",
  "directAffinityRate",
]);
const PLAIN_PATHS = new Set(["fistExpectation", "jiuqiangLayers"]);

const fieldCache = new Map<string, PanelField[]>();

/** Every number in the build's input the user can enter, in preset order. */
export function panelFields(build: AnyBuild): PanelField[] {
  const cached = fieldCache.get(build.id);
  if (cached != null) return cached;
  const fields: PanelField[] = [];
  const walk = (value: unknown, path: string[]) => {
    if (typeof value === "number") {
      const joined = path.join(".");
      const key = path.at(-1) ?? "";
      const group = path.length > 1 ? (path[0] ?? "") : "";
      const plain = PLAIN_KEY.test(key) || PLAIN_PATHS.has(joined);
      const raw = build.rawRates && group === "panelAttributes" && RAW_RATE_KEYS.has(key);
      fields.push({
        path: joined,
        group,
        label: fieldLabel(joined),
        percent: !plain && !raw,
        rawPercent: raw,
      });
    } else if (typeof value === "object" && value != null && !Array.isArray(value)) {
      if (path.length === 1 && SKIPPED_GROUPS.has(path[0] ?? "")) return;
      for (const [key, child] of Object.entries(value)) walk(child, [...path, key]);
    }
  };
  walk(build.preset, []);
  // Grouped, panel first; top-level numbers last.
  const order = Object.keys(GROUP_LABELS);
  fields.sort((a, b) => {
    const ga = order.indexOf(a.group);
    const gb = order.indexOf(b.group);
    return (ga < 0 ? order.length : ga) - (gb < 0 ? order.length : gb);
  });
  fieldCache.set(build.id, fields);
  return fields;
}

export function groupLabel(group: string): string {
  return GROUP_LABELS[group] ?? group;
}

// ---- Inputs ----------------------------------------------------------------

type Tree = Record<string, unknown>;

function readPath(tree: object, path: string): number | undefined {
  let node: unknown = tree;
  for (const key of path.split(".")) {
    if (typeof node !== "object" || node == null) return undefined;
    node = (node as Tree)[key];
  }
  return typeof node === "number" ? node : undefined;
}

/** Sets a number in an input built from a preset (so the path exists). */
export function setFieldValue(tree: object, path: string, value: number): void {
  const keys = path.split(".");
  const last = keys.pop();
  let node = tree as Tree;
  for (const key of keys) node = node[key] as Tree;
  if (last != null) node[last] = value;
}

export function currentBuild(settings: DpsSettings): AnyBuild {
  return buildById(settings.build);
}

/** The build's preset with the user's numbers laid over it. */
export function buildInput(build: AnyBuild, overrides: PanelOverrides = {}): object {
  const input = structuredClone(build.preset);
  for (const { path } of panelFields(build)) {
    const value = overrides[path];
    if (value != null) setFieldValue(input, path, value);
  }
  return input;
}

export function settingsInput(settings: DpsSettings): object {
  const build = currentBuild(settings);
  return buildInput(build, settings.panels[build.id]);
}

/** A field's value in an input. */
export function fieldValue(input: object, path: string): number {
  return readPath(input, path) ?? 0;
}

/** Coerces stored settings: unknown builds, levels and fields are dropped. */
export function normalizeDpsSettings(raw: unknown): DpsSettings {
  const source = typeof raw === "object" && raw != null ? (raw as Tree) : {};
  const settings = defaultDpsSettings();
  if (typeof source.build === "string" && BUILDS.some(({ id }) => id === source.build)) {
    settings.build = source.build;
  }
  if (typeof source.level === "number" && LEVELS.includes(source.level))
    settings.level = source.level;
  const panels =
    typeof source.panels === "object" && source.panels != null ? (source.panels as Tree) : {};
  for (const build of BUILDS) {
    const entered = panels[build.id];
    if (typeof entered !== "object" || entered == null) continue;
    const kept: PanelOverrides = {};
    for (const { path } of panelFields(build)) {
      const value = (entered as Tree)[path];
      if (typeof value === "number" && Number.isFinite(value)) kept[path] = value;
    }
    if (Object.keys(kept).length > 0) settings.panels[build.id] = kept;
  }
  return settings;
}

// ---- Evaluation ------------------------------------------------------------

export type DpsResult = {
  dps: number;
  // DPS against the level's 100%-毕业 baseline, as a percentage.
  graduationRate: number;
};

export function evaluate(build: AnyBuild, input: object, level: number): DpsResult {
  const dps = build.dps(input);
  const byLevel = BASELINE_DPS[build.baselineKey] ?? {};
  const baseline = byLevel[level] ?? byLevel[DEFAULT_LEVEL] ?? 1;
  return { dps, graduationRate: (dps / baseline) * 100 };
}

/** Relative DPS change from `before` to `after`, as a percentage. */
export function dpsChangePercent(before: DpsResult, after: DpsResult): number {
  return before.dps > 0 ? ((after.dps - before.dps) / before.dps) * 100 : 0;
}

// ---- 词条 ------------------------------------------------------------------

/** A 词条's panel changes per point of the value the game shows (5.2 for "+5.2%"). */
type AffixEffect = {
  rollKey: string;
  apply: (value: number, rate: (value: number) => number, build: AnyBuild) => [string, number][];
};

const attack = (side: "Min" | "Max", element: string): AffixEffect => ({
  rollKey:
    element === "wuxiang"
      ? `${side.toLowerCase()}WuxiangAttack`
      : `${side.toLowerCase()}ElementAttack`,
  apply: (v) => [[`panelAttributes.${element}Attack${side}`, v]],
});

const AFFIX_EFFECTS: Partial<Record<Affix, AffixEffect>> = {
  劲: {
    rollKey: "strength",
    apply: (v) => [
      ["panelAttributes.physicalAttackMin", v * CONVERSION_RATES.strengthToMinAttack],
      ["panelAttributes.physicalAttackMax", v * CONVERSION_RATES.strengthToMaxAttack],
    ],
  },
  敏: {
    rollKey: "agility",
    apply: (v, rate) => [
      ["panelAttributes.physicalAttackMin", v * CONVERSION_RATES.agilityToMinAttack],
      ["panelAttributes.criticalRate", rate(v * CONVERSION_RATES.agilityToCriticalRate)],
    ],
  },
  势: {
    rollKey: "momentum",
    apply: (v, rate) => [
      ["panelAttributes.physicalAttackMax", v * CONVERSION_RATES.momentumToAttack],
      ["panelAttributes.affinityRate", rate(v * CONVERSION_RATES.momentumToAffinityRate)],
    ],
  },
  最大外功攻击: {
    rollKey: "maxPhysicalAttack",
    apply: (v) => [["panelAttributes.physicalAttackMax", v]],
  },
  最小外功攻击: {
    rollKey: "minPhysicalAttack",
    apply: (v) => [["panelAttributes.physicalAttackMin", v]],
  },
  精准率: {
    rollKey: "precisionRate",
    apply: (v, rate) => [["panelAttributes.precisionRate", rate(v)]],
  },
  会心率: {
    rollKey: "criticalRate",
    apply: (v, rate) => [["panelAttributes.criticalRate", rate(v)]],
  },
  会意率: {
    rollKey: "affinityRate",
    apply: (v, rate) => [["panelAttributes.affinityRate", rate(v)]],
  },
  最大鸣金攻击: attack("Max", "bellstrike"),
  最小鸣金攻击: attack("Min", "bellstrike"),
  最大裂石攻击: attack("Max", "stonesplit"),
  最小裂石攻击: attack("Min", "stonesplit"),
  最大牵丝攻击: attack("Max", "qiansi"),
  最小牵丝攻击: attack("Min", "qiansi"),
  最大破竹攻击: attack("Max", "pozhu"),
  最小破竹攻击: attack("Min", "pozhu"),
  最大无相攻击: attack("Max", "wuxiang"),
  最小无相攻击: attack("Min", "wuxiang"),
  // In game this is one weapon's 武学增效; the build's main weapon is assumed.
  武学增效: {
    rollKey: "martialArtsDamageBonus",
    apply: (v, _rate, build) => [[`weaponBonus.${build.mainWeapon}`, v / 100]],
  },
  全武学增效: {
    rollKey: "allWeaponDamageBonus",
    apply: (v) => [["weaponBonus.allWeapon", v / 100]],
  },
  首领增伤: { rollKey: "bossDamageBonus", apply: (v) => [["weaponBonus.boss", v / 100]] },
  单体奇术增伤: {
    rollKey: "skillDamageBonus",
    apply: (v) => [["skillBonus.singleTarget", v / 100]],
  },
  群体奇术增伤: { rollKey: "skillDamageBonus", apply: (v) => [["skillBonus.aoe", v / 100]] },
};

/** The panel changes `amount` of `affix` makes, limited to fields the build has. */
function affixChanges(
  build: AnyBuild,
  input: object,
  affix: Affix,
  amount: number,
): [string, number][] {
  const effect = AFFIX_EFFECTS[affix];
  if (effect == null) return [];
  // Gear rates reach a final-fraction panel divided by the target's judgment resistance.
  const resistance = targetOf(String((input as Tree).targetType ?? "")).judgmentResistance;
  const rate = build.rawRates ? (v: number) => v : (v: number) => v / 100 / resistance;
  return effect.apply(amount, rate, build).filter(([path]) => readPath(build.preset, path) != null);
}

/** Whether `affix` moves any number the build reads. */
export function affixHasEffect(build: AnyBuild, affix: Affix): boolean {
  return affixChanges(build, build.preset, affix, 1).length > 0;
}

/** The equipment tier whose roll ranges apply at `level`, e.g. 95 → 91. */
function equipmentTier(level: number): number {
  const tiers = Object.keys(MAX_ROLLS)
    .map(Number)
    .sort((a, b) => a - b);
  return tiers.filter((tier) => tier <= level).at(-1) ?? tiers[0] ?? DEFAULT_LEVEL;
}

/** The best roll of `affix` at `level`'s equipment tier, or 0 when unpriced. */
export function maxRoll(affix: Affix, level: number): number {
  const effect = AFFIX_EFFECTS[affix];
  if (effect == null) return 0;
  return MAX_ROLLS[equipmentTier(level)]?.[effect.rollKey] ?? 0;
}

/**
 * The number in a free-form rolled value like "+5.2%" or "57.4", or the tier's
 * best roll when the value is blank or has no number.
 */
export function rolledAmount(affix: Affix, value: string, level: number): number {
  const match = /-?\d+(?:\.\d+)?/.exec(value);
  const parsed = match == null ? Number.NaN : Number(match[0]);
  return Number.isFinite(parsed) ? parsed : maxRoll(affix, level);
}

/** A 词条 with its amount, as it affects the panel. */
export type PricedAffix = { affix: Affix; amount: number };

/** `input` with `removed` taken off and `added` put on. */
export function swapAffixes(
  build: AnyBuild,
  input: object,
  removed: readonly PricedAffix[],
  added: readonly PricedAffix[],
): object {
  const next = structuredClone(input);
  const apply = ({ affix, amount }: PricedAffix, sign: number) => {
    for (const [path, delta] of affixChanges(build, input, affix, amount)) {
      setFieldValue(next, path, fieldValue(next, path) + sign * delta);
    }
  };
  for (const affix of removed) apply(affix, -1);
  for (const affix of added) apply(affix, 1);
  return next;
}
