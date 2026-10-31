// The 燕云十六声 gear tracker's whole state, persisted as one JSON document per
// user in the shared random_small_apps table. The game's 词条 and 部位 are
// hardcoded below; the user sets how many of each 词条 the whole set needs
// (毕业要求) and enters their own pieces — the app counts, and searches for the
// best set to wear.

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

/** A piece's 词条 positions, in order: 宫 is the initial one, the rest are tuned. */
export const AFFIX_POSITIONS = ["宫", "商", "角", "徵", "羽"] as const;

export const MAX_AFFIXES = AFFIX_POSITIONS.length;

/** The most any one 词条 could ever be needed: every position of every slot. */
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

export type WwmGearTrackerData = {
  version: 2;
  quotas: Quotas;
  gears: Gear[];
};

export function newId(): string {
  return crypto.randomUUID();
}

export function emptyWwmGearTrackerData(): WwmGearTrackerData {
  return { version: 2, quotas: {}, gears: [] };
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
  return { version: 2, quotas, gears };
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
 * `needed` of each 词条 across the set, in slot then position order. The rest
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

export type OptimalSet = {
  // One piece for every slot that has any piece, in slot order.
  gears: Gear[];
  // False when the search hit its budget and `gears` is only the best found.
  exhaustive: boolean;
};

// Search nodes visited before giving up on proving optimality.
const SEARCH_BUDGET = 1_000_000;

/**
 * The set — one piece per slot that has any — that fills the most quota.
 * Among equally good sets it keeps the most currently worn pieces, then
 * prefers more useful 词条 overall. A depth-first search over slots, pruned
 * by the most quota the remaining slots could still fill.
 */
export function optimalSet({ gears, quotas }: WwmGearTrackerData): OptimalSet {
  // Only needed 词条 matter; every vector below is indexed like `needed`.
  const tracked = AFFIXES.filter((affix) => quotaOf(quotas, affix) > 0);
  const needed = tracked.map((affix) => quotaOf(quotas, affix));
  const trackedIndex = new Map(tracked.map((affix, i) => [affix, i]));
  type Candidate = { gear: Gear; counts: number[]; useful: number };
  const toCandidate = (gear: Gear): Candidate => {
    const counts = tracked.map(() => 0);
    for (const { affix } of gear.affixes) {
      const i = trackedIndex.get(affix);
      if (i != null) counts[i] = (counts[i] ?? 0) + 1;
    }
    return { gear, counts, useful: counts.reduce((sum, n) => sum + n, 0) };
  };

  const slots: Candidate[][] = [];
  for (const slot of GEAR_SLOTS) {
    const candidates = gears.filter((gear) => gear.slot === slot).map(toCandidate);
    if (candidates.length === 0) continue;
    // Likely-best first, so the bound prunes early.
    candidates.sort(
      (a, b) => b.useful - a.useful || Number(b.gear.equipped) - Number(a.gear.equipped),
    );
    slots.push(candidates);
  }

  // suffixMax[i][a]: the most of 词条 `a` slots i.. could add together.
  const suffixMax: number[][] = [tracked.map(() => 0)];
  for (const candidates of [...slots].reverse()) {
    const next = suffixMax[0] ?? [];
    suffixMax.unshift(
      next.map((sum, a) => sum + Math.max(...candidates.map(({ counts }) => counts[a] ?? 0))),
    );
  }

  const counts = tracked.map(() => 0);
  const choice: Candidate[] = [];
  let best: { filled: number; kept: number; useful: number; gears: Gear[] } | null = null;
  let budget = SEARCH_BUDGET;
  let truncated = false;

  const filledWith = (extra: readonly number[]): number => {
    let filled = 0;
    needed.forEach((quota, a) => {
      filled += Math.min(quota, (counts[a] ?? 0) + (extra[a] ?? 0));
    });
    return filled;
  };
  const add = (delta: readonly number[], sign: 1 | -1): void => {
    delta.forEach((n, a) => {
      counts[a] = (counts[a] ?? 0) + sign * n;
    });
  };

  const search = (i: number, kept: number, useful: number): void => {
    if (budget <= 0) {
      truncated = true;
      return;
    }
    budget--;
    const candidates = slots[i];
    if (candidates == null) {
      const filled = filledWith([]);
      if (
        best == null ||
        filled > best.filled ||
        (filled === best.filled &&
          (kept > best.kept || (kept === best.kept && useful > best.useful)))
      ) {
        best = { filled, kept, useful, gears: choice.map(({ gear }) => gear) };
      }
      return;
    }
    if (best != null && filledWith(suffixMax[i] ?? []) < best.filled) return;
    for (const candidate of candidates) {
      add(candidate.counts, 1);
      choice.push(candidate);
      search(i + 1, kept + Number(candidate.gear.equipped), useful + candidate.useful);
      choice.pop();
      add(candidate.counts, -1);
    }
  };
  search(0, 0, 0);

  // Assigned only inside `search`, which TypeScript does not follow.
  const found = best as { gears: Gear[] } | null;
  return { gears: found?.gears ?? [], exhaustive: !truncated };
}
