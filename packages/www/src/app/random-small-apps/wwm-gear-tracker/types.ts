// The 燕云十六声 gear tracker's whole state, persisted as one JSON document per
// user in the shared random_small_apps table. The game's 词条 and 部位 are
// hardcoded below; the user sets how many of each 词条 the whole set needs
// (毕业要求), enters their own pieces, and configures the set they aim for
// (目标配装) — the app counts, and compares what is worn against that target.

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

/** The eight 部位: one piece of each is worn at a time. */
export const GEAR_SLOTS = ["主武器", "副武器", "环", "佩", "头盔", "胸甲", "胫甲", "腕甲"] as const;

export type GearSlot = (typeof GEAR_SLOTS)[number];

function isGearSlot(value: unknown): value is GearSlot {
  return typeof value === "string" && (GEAR_SLOTS as readonly string[]).includes(value);
}

/** The most 词条 a piece can have. Their order on a piece does not matter. */
export const MAX_AFFIXES = 5;

/** The most any one 词条 could ever be needed: every 词条 of every slot. */
export const MAX_QUOTA = GEAR_SLOTS.length * MAX_AFFIXES;

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
 * How many of each 词条 the whole worn set needs. A quota is shared across
 * every piece: rolled 词条 from all worn pieces fill it together, and extra
 * 词条 of one kind never compensate for a shortfall in another. 词条 missing
 * here are not needed at all.
 */
export type Quotas = Partial<Record<Affix, number>>;

/**
 * The set the user aims for: the 词条 each slot's piece should have, at most
 * MAX_AFFIXES per slot. Order within a slot does not matter.
 */
export type TargetSet = Partial<Record<GearSlot, Affix[]>>;

export type WwmGearTrackerData = {
  version: 2;
  quotas: Quotas;
  target: TargetSet;
  gears: Gear[];
};

export function newId(): string {
  return crypto.randomUUID();
}

export function emptyWwmGearTrackerData(): WwmGearTrackerData {
  return { version: 2, quotas: {}, target: {}, gears: [] };
}

function normalizeName(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeId(value: unknown): string {
  return typeof value === "string" && value !== "" ? value : newId();
}

export function clampQuota(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(MAX_QUOTA, Math.max(0, Math.round(value)));
}

export function quotaOf(quotas: Quotas, affix: Affix): number {
  return quotas[affix] ?? 0;
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
 * match a real 词条; their old category quotas do not carry over.
 */
export function normalizeWwmGearTrackerData(raw: unknown): WwmGearTrackerData {
  if (typeof raw !== "object" || raw == null) return emptyWwmGearTrackerData();
  const source = raw as Record<string, unknown>;

  const quotas: Quotas = {};
  if (typeof source.quotas === "object" && source.quotas != null) {
    for (const [affix, needed] of Object.entries(source.quotas as Record<string, unknown>)) {
      if (!isAffix(affix) || typeof needed !== "number") continue;
      const quota = clampQuota(needed);
      if (quota > 0) quotas[affix] = quota;
    }
  }

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
  return { version: 2, quotas, target, gears };
}

/** Pieces in slot order, keeping entry order within a slot. */
export function sortedBySlot(gears: readonly Gear[]): Gear[] {
  return [...gears].sort((a, b) => GEAR_SLOTS.indexOf(a.slot) - GEAR_SLOTS.indexOf(b.slot));
}

export function equippedGears({ gears }: WwmGearTrackerData): Gear[] {
  return sortedBySlot(gears.filter(({ equipped }) => equipped));
}

/**
 * The ids of rolled 词条 that count toward their quota within `set`: the first
 * `needed` of each 词条 across the set, in slot then entry order. The rest
 * are overfills and render as uncounted.
 */
export function countedAffixIds(set: readonly Gear[], quotas: Quotas): ReadonlySet<string> {
  const seen = new Map<Affix, number>();
  const counted = new Set<string>();
  for (const gear of sortedBySlot(set)) {
    for (const rolled of gear.affixes) {
      const n = (seen.get(rolled.affix) ?? 0) + 1;
      seen.set(rolled.affix, n);
      if (n <= quotaOf(quotas, rolled.affix)) counted.add(rolled.id);
    }
  }
  return counted;
}

/** How one needed 词条 stands across a set. */
export type AffixFill = {
  affix: Affix;
  needed: number;
  // How many of this 词条 the set rolled.
  rolled: number;
  // Rolled ones that count: capped at the quota.
  counted: number;
};

/** Each needed 词条's quota, filled by the 词条 rolled across `set`. */
export function affixFills(set: readonly Gear[], quotas: Quotas): AffixFill[] {
  const rolledBy = affixUsage(set);
  const fills: AffixFill[] = [];
  for (const affix of AFFIXES) {
    const needed = quotaOf(quotas, affix);
    if (needed <= 0) continue;
    const rolled = rolledBy.get(affix) ?? 0;
    fills.push({ affix, needed, rolled, counted: Math.min(rolled, needed) });
  }
  return fills;
}

export type SetStats = {
  pieces: number;
  // Total quota across 词条 — defined once, shared by every piece.
  required: number;
  // Quota filled by the set's rolled 词条 together.
  filled: number;
  // Whether every quota is met.
  graduated: boolean;
  // How many more counted 词条 are needed to graduate.
  remaining: number;
};

export function setStats(set: readonly Gear[], quotas: Quotas): SetStats {
  let required = 0;
  let filled = 0;
  for (const { needed, counted } of affixFills(set, quotas)) {
    required += needed;
    filled += counted;
  }
  return {
    pieces: set.length,
    required,
    filled,
    graduated: required > 0 && filled >= required,
    remaining: Math.max(0, required - filled),
  };
}

export type GearStats = {
  rolled: number;
  // Rolled 词条 the build needs at all, whether or not their quota has room.
  useful: number;
  // Rolled 词条 that count toward the quotas within the worn set.
  counted: number;
};

export function gearStats(gear: Gear, quotas: Quotas, countedIds: ReadonlySet<string>): GearStats {
  let useful = 0;
  let counted = 0;
  for (const rolled of gear.affixes) {
    if (quotaOf(quotas, rolled.affix) > 0) useful++;
    if (countedIds.has(rolled.id)) counted++;
  }
  return { rolled: gear.affixes.length, useful, counted };
}

/** A share as a percentage like "37.5%", or "—" when there is nothing to measure. */
export function formatRate(numerator: number, denominator: number): string {
  if (denominator <= 0) return "—";
  const pct = Math.min(1, numerator / denominator) * 100;
  const rounded = Math.round(pct * 10) / 10;
  return `${rounded}%`;
}

/** How many times each 词条 was rolled across `set`. */
export function affixUsage(set: readonly Gear[]): Map<Affix, number> {
  const usage = new Map<Affix, number>();
  for (const gear of set) {
    for (const { affix } of gear.affixes) usage.set(affix, (usage.get(affix) ?? 0) + 1);
  }
  return usage;
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
  const unmatched = new Map<Affix, number>();
  for (const affix of target) unmatched.set(affix, (unmatched.get(affix) ?? 0) + 1);
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

/** How many of each 词条 the target set has across all slots. */
export function targetUsage(target: TargetSet): Map<Affix, number> {
  const usage = new Map<Affix, number>();
  for (const affixes of Object.values(target)) {
    for (const affix of affixes) usage.set(affix, (usage.get(affix) ?? 0) + 1);
  }
  return usage;
}
