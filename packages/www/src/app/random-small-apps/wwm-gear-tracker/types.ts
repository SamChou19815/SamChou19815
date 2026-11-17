// The 燕云十六声 gear tracker's whole state, persisted as one JSON document per
// user in the shared random_small_apps table. The game's 词条 and 部位 are
// hardcoded below; the user enters their own pieces and tracks any number of
// gear sets (配装), each with the set it aims for (目标配装), the pieces it
// wears and its DPS panel. Pieces are shared: several sets can wear one. Within
// a set, the target set is the only source of truth for what
// is needed: across all slots, it asks for some number of each 词条. The worn
// set satisfies those counts together, wherever its 词条 sit; extra 词条 of one
// kind never make up for a shortfall in another.

import { defaultDpsSettings, normalizeDpsSettings, type DpsSettings } from "./dpsModel";

/** Every 词条 a piece can roll, grouped as the game's guides group them. */
export const AFFIX_GROUPS = [
  { name: "五维", affixes: ["劲", "敏", "势", "体", "御"] },
  { name: "基础", affixes: ["最大外功攻击", "最小外功攻击", "气血最大值", "外功防御"] },
  { name: "三率", affixes: ["精准率", "会心率", "会意率"] },
  {
    name: "属攻",
    affixes: [
      "最大鸣金攻击",
      "最小鸣金攻击",
      "最大裂石攻击",
      "最小裂石攻击",
      "最大牵丝攻击",
      "最小牵丝攻击",
      "最大破竹攻击",
      "最小破竹攻击",
      // Adaptive: becomes the 属攻 of whichever school is being played.
      "最大无相攻击",
      "最小无相攻击",
    ],
  },
  {
    name: "神力",
    affixes: ["武学增效", "全武学增效", "首领增伤", "单体奇术增伤", "群体奇术增伤", "玩家单位增伤"],
  },
] as const;

export type Affix = (typeof AFFIX_GROUPS)[number]["affixes"][number];

export const AFFIXES: readonly Affix[] = AFFIX_GROUPS.flatMap(({ affixes }) => affixes);

const AFFIX_SET: ReadonlySet<string> = new Set(AFFIXES);

function isAffix(value: unknown): value is Affix {
  return typeof value === "string" && AFFIX_SET.has(value);
}

/**
 * The eight 部位: one piece of each is worn at a time. Listed as the game lays
 * them out — the left block (主武器 副武器 over 环 佩) before the right block
 * (头盔 胸甲 over 胫甲 腕甲).
 */
export const GEAR_SLOTS = ["主武器", "副武器", "环", "佩", "头盔", "胸甲", "胫甲", "腕甲"] as const;

export type GearSlot = (typeof GEAR_SLOTS)[number];

function isGearSlot(value: unknown): value is GearSlot {
  return typeof value === "string" && (GEAR_SLOTS as readonly string[]).includes(value);
}

/** The most 词条 a piece can have. Their order on a piece does not matter. */
export const MAX_AFFIXES = 5;

/** One rolled 词条 on a piece, plus its rolled value. */
export type RolledAffix = {
  id: string;
  affix: Affix;
  // Free-form display value, e.g. "+5.2%" — shown as entered, never interpreted.
  value: string;
};

/** One gear piece the user is tracking. */
export type Gear = {
  id: string;
  name: string;
  slot: GearSlot;
  // Whether the piece is worn. At most one piece per slot is.
  equipped: boolean;
  // How many 词条 the piece has in total, rolled or not, up to MAX_AFFIXES.
  affixCount: number;
  affixes: RolledAffix[];
};

/**
 * The set the user aims for: the 词条 each slot's piece should have, at most
 * MAX_AFFIXES per slot. Order within a slot does not matter.
 */
export type TargetSet = Partial<Record<GearSlot, Affix[]>>;

/**
 * One gear set as the rest of the app sees it: its target, every piece (marked
 * worn when this set wears it) and its DPS settings. Derived from the stored
 * document with `setView` and written back with `applySetView`.
 */
export type WwmGearTrackerData = {
  target: TargetSet;
  gears: Gear[];
  // The build, level and in-game panel (with the worn set on) that every 词条
  // change is priced against.
  dps: DpsSettings;
};

/** A piece as stored: which sets wear it is kept on the sets. */
export type StoredGear = Omit<Gear, "equipped">;

/** One gear set (配装): what it aims for, what it wears and how it is priced. */
export type GearSet = {
  id: string;
  name: string;
  target: TargetSet;
  // Ids of the pieces this set wears, at most one per slot.
  worn: string[];
  dps: DpsSettings;
};

/**
 * The persisted document: any number of gear sets sharing one inventory of
 * pieces, so a piece can be worn by several sets.
 */
export type WwmGearTrackerDocument = {
  version: 5;
  activeSetId: string;
  sets: GearSet[];
  gears: StoredGear[];
};

export function newId(): string {
  return crypto.randomUUID();
}

/** The name a set gets when it has none, e.g. "配装 2". */
export function defaultSetName(index: number): string {
  return `配装 ${index + 1}`;
}

export function emptyGearSet(name: string, level?: number): GearSet {
  const dps = defaultDpsSettings();
  if (level != null) dps.level = level;
  return { id: newId(), name, target: {}, worn: [], dps };
}

export function emptyWwmGearTrackerDocument(): WwmGearTrackerDocument {
  const set = emptyGearSet(defaultSetName(0));
  return { version: 5, activeSetId: set.id, sets: [set], gears: [] };
}

function normalizeName(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeId(value: unknown): string {
  return typeof value === "string" && value !== "" ? value : newId();
}

/**
 * Makes `winnerId` the only worn piece in its slot, or, without a winner,
 * keeps only the first worn piece of each slot.
 */
export function enforceSingleEquipped(gears: Gear[], winnerId: string | null = null): void {
  const winner = winnerId == null ? null : gears.find(({ id }) => id === winnerId);
  const taken = new Set<GearSlot>();
  if (winner?.equipped === true) taken.add(winner.slot);
  for (const gear of gears) {
    if (!gear.equipped || gear === winner) continue;
    if (taken.has(gear.slot)) gear.equipped = false;
    else taken.add(gear.slot);
  }
}

function normalizeTarget(raw: unknown): TargetSet {
  const target: TargetSet = {};
  if (typeof raw !== "object" || raw == null) return target;
  for (const [slot, affixes] of Object.entries(raw as Record<string, unknown>)) {
    if (!isGearSlot(slot) || !Array.isArray(affixes)) continue;
    const valid = affixes.filter(isAffix).slice(0, MAX_AFFIXES);
    if (valid.length > 0) target[slot] = valid;
  }
  return target;
}

/**
 * Well-formed pieces from whatever `rawGears` holds, with duplicate ids
 * replaced. `legacyNames` maps version 1's affix pool ids to their names.
 * Pieces without a stored `equipped` are returned in `unknownEquipped`.
 */
function normalizeGears(
  rawGears: unknown,
  legacyNames: ReadonlyMap<string, string> = new Map(),
): { gears: Gear[]; unknownEquipped: Set<Gear> } {
  const gears: Gear[] = [];
  const unknownEquipped = new Set<Gear>();
  const seenIds = new Set<string>();
  if (Array.isArray(rawGears)) {
    for (const entry of rawGears) {
      if (typeof entry !== "object" || entry == null) continue;
      const gear = entry as Record<string, unknown>;
      const affixes: RolledAffix[] = [];
      if (Array.isArray(gear.affixes)) {
        for (const rolled of gear.affixes) {
          if (typeof rolled !== "object" || rolled == null) continue;
          const { id, affix, definitionId, value } = rolled as Record<string, unknown>;
          const name =
            affix ?? (typeof definitionId === "string" ? legacyNames.get(definitionId) : null);
          if (!isAffix(name)) continue;
          affixes.push({ id: normalizeId(id), affix: name, value: normalizeName(value) });
        }
      }
      const rawCount = typeof gear.affixCount === "number" ? gear.affixCount : Number.NaN;
      const affixCount = Number.isFinite(rawCount)
        ? Math.min(MAX_AFFIXES, Math.max(1, Math.round(rawCount)))
        : MAX_AFFIXES;
      let id = normalizeId(gear.id);
      if (seenIds.has(id)) id = newId();
      seenIds.add(id);
      const normalized: Gear = {
        id,
        name: normalizeName(gear.name),
        slot: isGearSlot(gear.slot) ? gear.slot : GEAR_SLOTS[0],
        equipped: gear.equipped === true,
        affixCount,
        affixes: affixes.slice(0, affixCount),
      };
      if (typeof gear.equipped !== "boolean") unknownEquipped.add(normalized);
      gears.push(normalized);
    }
  }
  return { gears, unknownEquipped };
}

/**
 * Coerces a document from before version 5 — a single set, with the worn
 * pieces marked on the pieces themselves — into one set's data. 词条 and 部位
 * the game does not have are dropped. Documents from the first version, which
 * kept a user-defined affix pool, keep the rolled affixes whose names match a
 * real 词条. The per-词条 quotas of earlier versions are dropped: the target
 * set replaced them.
 */
function normalizeSingleSetData(source: Record<string, unknown>): WwmGearTrackerData {
  // Version 1 referenced its own pool entries by id.
  const legacyNames = new Map<string, string>();
  if (Array.isArray(source.affixPool)) {
    for (const entry of source.affixPool) {
      if (typeof entry !== "object" || entry == null) continue;
      const { id, name } = entry as Record<string, unknown>;
      if (typeof id === "string") legacyNames.set(id, normalizeName(name));
    }
  }

  const { gears, unknownEquipped } = normalizeGears(source.gears, legacyNames);
  enforceSingleEquipped(gears);
  // Pieces saved before `equipped` existed: the first of each slot is worn.
  for (const gear of unknownEquipped) {
    if (!gears.some((other) => other.equipped && other.slot === gear.slot)) gear.equipped = true;
  }
  // Documents before version 4 had no DPS settings: they start from the preset.
  return {
    target: normalizeTarget(source.target),
    gears,
    dps: normalizeDpsSettings(source.dps),
  };
}

function storedGear({ id, name, slot, affixCount, affixes }: Gear): StoredGear {
  return { id, name, slot, affixCount, affixes };
}

/**
 * `ids` limited to pieces in `gears`, without repeats and with at most one per
 * slot — the first one listed wins.
 */
function validWorn(gears: readonly StoredGear[], ids: readonly unknown[]): string[] {
  const slotOf = new Map(gears.map(({ id, slot }) => [id, slot]));
  const taken = new Set<GearSlot>();
  const worn: string[] = [];
  for (const id of ids) {
    if (typeof id !== "string") continue;
    const slot = slotOf.get(id);
    if (slot == null || taken.has(slot)) continue;
    taken.add(slot);
    worn.push(id);
  }
  return worn;
}

/**
 * Coerces whatever the row holds (or nothing) into a well-formed document.
 * Documents from before version 5 become a document with one set.
 */
export function normalizeWwmGearTrackerDocument(raw: unknown): WwmGearTrackerDocument {
  if (typeof raw !== "object" || raw == null) return emptyWwmGearTrackerDocument();
  const source = raw as Record<string, unknown>;

  if (!Array.isArray(source.sets)) {
    const single = normalizeSingleSetData(source);
    const gears = single.gears.map(storedGear);
    const set: GearSet = {
      // Fixed, so normalizing the same legacy row twice gives the same document.
      id: "default",
      name: defaultSetName(0),
      target: single.target,
      worn: single.gears.filter(({ equipped }) => equipped).map(({ id }) => id),
      dps: single.dps,
    };
    return { version: 5, activeSetId: set.id, sets: [set], gears };
  }

  const gears = normalizeGears(source.gears).gears.map(storedGear);
  const sets: GearSet[] = [];
  const seenIds = new Set<string>();
  for (const entry of source.sets) {
    if (typeof entry !== "object" || entry == null) continue;
    const set = entry as Record<string, unknown>;
    let id = normalizeId(set.id);
    if (seenIds.has(id)) id = newId();
    seenIds.add(id);
    const name = normalizeName(set.name);
    sets.push({
      id,
      name: name === "" ? defaultSetName(sets.length) : name,
      target: normalizeTarget(set.target),
      worn: validWorn(gears, Array.isArray(set.worn) ? set.worn : []),
      dps: normalizeDpsSettings(set.dps),
    });
  }
  if (sets.length === 0) sets.push(emptyGearSet(defaultSetName(0)));
  const activeSetId = sets.find(({ id }) => id === source.activeSetId)?.id ?? sets[0]?.id ?? "";
  return { version: 5, activeSetId, sets, gears };
}

/** The set being viewed: the active one, or the first if it is gone. */
export function activeSet(doc: WwmGearTrackerDocument): GearSet {
  const set = doc.sets.find(({ id }) => id === doc.activeSetId) ?? doc.sets[0];
  if (set == null) throw new Error("A gear tracker document always has a set.");
  return set;
}

/** A fresh copy of one set's data, with the pieces it wears marked worn. */
export function setView(doc: WwmGearTrackerDocument, set: GearSet): WwmGearTrackerData {
  const worn = new Set(set.worn);
  return {
    target: structuredClone(set.target),
    gears: doc.gears.map((gear) => ({
      ...gear,
      affixes: gear.affixes.map((rolled) => ({ ...rolled })),
      equipped: worn.has(gear.id),
    })),
    dps: structuredClone(set.dps),
  };
}

/**
 * `doc` with set `setId` replaced by `view`. Pieces are shared, so `view`'s
 * pieces become every set's; other sets drop pieces that are gone or now clash
 * in a slot. The character level is shared too.
 */
export function applySetView(
  doc: WwmGearTrackerDocument,
  setId: string,
  view: WwmGearTrackerData,
): WwmGearTrackerDocument {
  const gears = view.gears.map(storedGear);
  const wornHere = view.gears.filter(({ equipped }) => equipped).map(({ id }) => id);
  return {
    ...doc,
    gears,
    sets: doc.sets.map((set) =>
      set.id === setId
        ? { ...set, target: view.target, worn: validWorn(gears, wornHere), dps: view.dps }
        : {
            ...set,
            worn: validWorn(gears, set.worn),
            dps: { ...set.dps, level: view.dps.level },
          },
    ),
  };
}

/** Pieces in slot order, keeping entry order within a slot. */
export function sortedBySlot(gears: readonly Gear[]): Gear[] {
  return [...gears].sort((a, b) => GEAR_SLOTS.indexOf(a.slot) - GEAR_SLOTS.indexOf(b.slot));
}

export function equippedGears({ gears }: WwmGearTrackerData): Gear[] {
  return sortedBySlot(gears.filter(({ equipped }) => equipped));
}

/** How many times each 词条 appears in `affixes`. */
function countAffixes(affixes: Iterable<Affix>): Map<Affix, number> {
  const counts = new Map<Affix, number>();
  for (const affix of affixes) counts.set(affix, (counts.get(affix) ?? 0) + 1);
  return counts;
}

/** How many times each 词条 was rolled across `set`. */
export function affixUsage(set: readonly Gear[]): Map<Affix, number> {
  return countAffixes(set.flatMap(({ affixes }) => affixes.map(({ affix }) => affix)));
}

/** How many of each 词条 the target set has across all slots. */
export function targetUsage(target: TargetSet): Map<Affix, number> {
  return countAffixes(Object.values(target).flat());
}

/** How many of each 词条 one slot's target asks for. */
export function slotTargetUsage(target: TargetSet, slot: GearSlot): Map<Affix, number> {
  return countAffixes(target[slot] ?? []);
}

/** How one slot's worn piece stands against the target set's 词条 for it. */
export type SlotComparison = {
  slot: GearSlot;
  worn: Gear | null;
  target: Affix[];
  // Target 词条 the worn piece has, counted with multiplicity.
  matched: Affix[];
  // Target 词条 the worn piece lacks.
  missing: Affix[];
  // Worn 词条 the target does not ask for.
  extra: Affix[];
};

export function compareSlot(slot: GearSlot, worn: Gear | null, target: Affix[]): SlotComparison {
  const unmatched = countAffixes(target);
  const matched: Affix[] = [];
  const extra: Affix[] = [];
  for (const { affix } of worn?.affixes ?? []) {
    const left = unmatched.get(affix) ?? 0;
    if (left > 0) {
      unmatched.set(affix, left - 1);
      matched.push(affix);
    } else {
      extra.push(affix);
    }
  }
  const missing: Affix[] = [];
  for (const affix of target) {
    const left = unmatched.get(affix) ?? 0;
    if (left <= 0) continue;
    unmatched.set(affix, left - 1);
    missing.push(affix);
  }
  return { slot, worn, target, matched, missing, extra };
}

/** Every slot's worn piece against the target set, in slot order. */
export function compareToTarget(data: WwmGearTrackerData): SlotComparison[] {
  const worn = wornBySlot(data);
  return GEAR_SLOTS.map((slot) =>
    compareSlot(slot, worn.get(slot) ?? null, data.target[slot] ?? []),
  );
}

/**
 * The ids of the worn set's rolled 词条 that count toward graduation: the
 * first ones of each 词条, up to how many the target set asks for. Pieces are
 * checked most-matching first — the piece matching its slot's target best has
 * its 词条 counted before pieces that match worse — so the rest are surplus.
 */
export function countedAffixIds(data: WwmGearTrackerData): ReadonlySet<string> {
  const left = targetUsage(data.target);
  const counted = new Set<string>();
  // Only one piece is worn per slot, so each worn piece's matching against the
  // optimal one is its slot comparison's matched count.
  const worn = equippedGears(data)
    .map((gear) => ({
      gear,
      matched: compareSlot(gear.slot, gear, data.target[gear.slot] ?? []).matched.length,
    }))
    // Most matches first; the stable sort keeps slot order among equals.
    .sort((x, y) => y.matched - x.matched);
  for (const { gear } of worn) {
    for (const rolled of gear.affixes) {
      const n = left.get(rolled.affix) ?? 0;
      if (n <= 0) continue;
      left.set(rolled.affix, n - 1);
      counted.add(rolled.id);
    }
  }
  return counted;
}

/** How one target 词条 stands across the worn set. */
export type AffixFill = {
  affix: Affix;
  // How many the target set asks for, across all slots.
  needed: number;
  // How many the worn set rolled, anywhere.
  rolled: number;
  // How many of the needed ones are satisfied: rolled, capped at `needed`.
  counted: number;
};

/** Each target 词条, satisfied by the worn set's 词条 of that kind, in any slot. */
export function affixFills(data: WwmGearTrackerData): AffixFill[] {
  const neededBy = targetUsage(data.target);
  const rolledBy = affixUsage(equippedGears(data));
  return AFFIXES.filter((affix) => neededBy.has(affix)).map((affix) => {
    const needed = neededBy.get(affix) ?? 0;
    const rolled = rolledBy.get(affix) ?? 0;
    return { affix, needed, rolled, counted: Math.min(rolled, needed) };
  });
}

export type SetStats = {
  pieces: number;
  // How many 词条 the target set has in total.
  required: number;
  // How many of them the worn set satisfies, in any slot.
  filled: number;
  // Whether every target 词条 is met.
  graduated: boolean;
  // How many more satisfied 词条 are needed to graduate.
  remaining: number;
};

export function setStats(data: WwmGearTrackerData): SetStats {
  let required = 0;
  let filled = 0;
  for (const { needed, counted } of affixFills(data)) {
    required += needed;
    filled += counted;
  }
  return {
    pieces: equippedGears(data).length,
    required,
    filled,
    graduated: required > 0 && filled >= required,
    remaining: Math.max(0, required - filled),
  };
}

/** The worn piece of each slot. */
export function wornBySlot(data: WwmGearTrackerData): Map<GearSlot, Gear> {
  return new Map(equippedGears(data).map((gear) => [gear.slot, gear]));
}

/** A share as a percentage like "37.5%", or "—" when there is nothing to measure. */
export function formatRate(numerator: number, denominator: number): string {
  if (denominator <= 0) return "—";
  const pct = Math.min(1, numerator / denominator) * 100;
  const rounded = Math.round(pct * 10) / 10;
  return `${rounded}%`;
}
