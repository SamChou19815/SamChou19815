// The 燕云十六声 gear tracker's whole state, persisted as one JSON document per
// user in the shared random_small_apps table. The game's 词条 and 部位 are
// hardcoded below; the user configures the set they aim for (目标配装) and
// enters their own pieces. The target set is the only source of truth for what
// is needed: a worn piece's 词条 count toward graduation exactly when its
// slot's target asks for them.

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

export type WwmGearTrackerData = {
  version: 3;
  target: TargetSet;
  gears: Gear[];
};

export function newId(): string {
  return crypto.randomUUID();
}

export function emptyWwmGearTrackerData(): WwmGearTrackerData {
  return { version: 3, target: {}, gears: [] };
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

/**
 * Coerces whatever the row holds (or nothing) into well-formed data. 词条 and
 * 部位 the game does not have are dropped. Documents from the first version,
 * which kept a user-defined affix pool, keep the rolled affixes whose names
 * match a real 词条. The per-词条 quotas of earlier versions are dropped: the
 * target set replaced them.
 */
export function normalizeWwmGearTrackerData(raw: unknown): WwmGearTrackerData {
  if (typeof raw !== "object" || raw == null) return emptyWwmGearTrackerData();
  const source = raw as Record<string, unknown>;

  const target: TargetSet = {};
  if (typeof source.target === "object" && source.target != null) {
    for (const [slot, affixes] of Object.entries(source.target as Record<string, unknown>)) {
      if (!isGearSlot(slot) || !Array.isArray(affixes)) continue;
      const valid = affixes.filter(isAffix).slice(0, MAX_AFFIXES);
      if (valid.length > 0) target[slot] = valid;
    }
  }

  // Version 1 referenced its own pool entries by id.
  const legacyNames = new Map<string, string>();
  if (Array.isArray(source.affixPool)) {
    for (const entry of source.affixPool) {
      if (typeof entry !== "object" || entry == null) continue;
      const { id, name } = entry as Record<string, unknown>;
      if (typeof id === "string") legacyNames.set(id, normalizeName(name));
    }
  }

  const gears: Gear[] = [];
  // Pieces saved before `equipped` existed: the first of each slot is worn.
  const unknownEquipped = new Set<Gear>();
  if (Array.isArray(source.gears)) {
    for (const entry of source.gears) {
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
      const normalized: Gear = {
        id: normalizeId(gear.id),
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
  enforceSingleEquipped(gears);
  for (const gear of unknownEquipped) {
    if (!gears.some((other) => other.equipped && other.slot === gear.slot)) gear.equipped = true;
  }
  return { version: 3, target, gears };
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

/**
 * The ids of the rolled 词条 on `gear` that its slot's target asks for, each
 * target entry matched at most once. For a worn piece these count toward
 * graduation; for a spare one they would if it were worn.
 */
export function matchedAffixIds(gear: Gear, target: TargetSet): ReadonlySet<string> {
  const left = slotTargetUsage(target, gear.slot);
  const matched = new Set<string>();
  for (const rolled of gear.affixes) {
    const n = left.get(rolled.affix) ?? 0;
    if (n <= 0) continue;
    left.set(rolled.affix, n - 1);
    matched.add(rolled.id);
  }
  return matched;
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
  const worn = new Map(equippedGears(data).map((gear) => [gear.slot, gear]));
  return GEAR_SLOTS.map((slot) =>
    compareSlot(slot, worn.get(slot) ?? null, data.target[slot] ?? []),
  );
}

/** How one target 词条 stands across the worn set. */
export type AffixFill = {
  affix: Affix;
  // How many the target set asks for, across all slots.
  needed: number;
  // How many the worn set rolled, anywhere.
  rolled: number;
  // Rolled ones in a slot whose target asks for them: at most `needed`.
  counted: number;
};

/** Each target 词条, filled by the worn pieces whose slot targets it. */
export function affixFills(data: WwmGearTrackerData): AffixFill[] {
  const neededBy = targetUsage(data.target);
  const rolledBy = affixUsage(equippedGears(data));
  const countedBy = countAffixes(compareToTarget(data).flatMap(({ matched }) => matched));
  return AFFIXES.filter((affix) => neededBy.has(affix)).map((affix) => ({
    affix,
    needed: neededBy.get(affix) ?? 0,
    rolled: rolledBy.get(affix) ?? 0,
    counted: countedBy.get(affix) ?? 0,
  }));
}

export type SetStats = {
  pieces: number;
  // How many 词条 the target set has in total.
  required: number;
  // How many of them the worn set has, in the right slots.
  filled: number;
  // Whether every target 词条 is met.
  graduated: boolean;
  // How many more matching 词条 are needed to graduate.
  remaining: number;
};

export function setStats(data: WwmGearTrackerData): SetStats {
  let required = 0;
  let filled = 0;
  for (const { target, matched } of compareToTarget(data)) {
    required += target.length;
    filled += matched.length;
  }
  return {
    pieces: equippedGears(data).length,
    required,
    filled,
    graduated: required > 0 && filled >= required,
    remaining: Math.max(0, required - filled),
  };
}

/** A share as a percentage like "37.5%", or "—" when there is nothing to measure. */
export function formatRate(numerator: number, denominator: number): string {
  if (denominator <= 0) return "—";
  const pct = Math.min(1, numerator / denominator) * 100;
  const rounded = Math.round(pct * 10) / 10;
  return `${rounded}%`;
}
