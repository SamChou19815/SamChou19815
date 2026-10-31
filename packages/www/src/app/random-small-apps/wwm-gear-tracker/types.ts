// The WWM gear tracker's whole state, persisted as one JSON document per user
// in the shared random_small_apps table. Nothing about the game is baked in:
// the user defines categories of affixes and how many of each their gear needs,
// files affixes into those categories, and enters their own pieces — the app
// only counts.

/**
 * A category of affixes with a quota. The quota is shared across every piece:
 * rolled affixes from all pieces fill it together, and extra affixes from one
 * category never compensate for a shortfall in another.
 */
export type AffixCategory = {
  id: string;
  name: string;
  needed: number;
};

/** An affix the user has defined, pickable when entering a piece. */
export type AffixDefinition = {
  id: string;
  name: string;
  // The category this affix fills, or null when it never counts.
  categoryId: string | null;
};

/** One rolled affix on a piece: a pool entry plus its rolled value. */
export type RolledAffix = {
  id: string;
  definitionId: string;
  // Free-form display value, e.g. "+5.2%" — shown as entered, never interpreted.
  value: string;
};

/** One gear piece the user is tracking. */
export type Gear = {
  id: string;
  name: string;
  // Free-form slot label, e.g. "ring" — optional.
  slot: string;
  // How many affixes the piece has in total, rolled or not. Display-only: the
  // graduation rate is measured against the shared category quotas, not this.
  affixCount: number;
  affixes: RolledAffix[];
};

export type WwmGearTrackerData = {
  version: 1;
  affixCategories: AffixCategory[];
  affixPool: AffixDefinition[];
  gears: Gear[];
};

export function newId(): string {
  return crypto.randomUUID();
}

export function emptyWwmGearTrackerData(): WwmGearTrackerData {
  return { version: 1, affixCategories: [], affixPool: [], gears: [] };
}

function normalizeName(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function normalizeId(value: unknown): string {
  return typeof value === "string" && value !== "" ? value : newId();
}

function normalizeNeeded(value: unknown): number {
  const num = typeof value === "number" ? value : Number.NaN;
  if (!Number.isFinite(num)) return 1;
  return Math.min(99, Math.max(0, Math.round(num)));
}

/**
 * Coerces whatever the row holds (or nothing) into well-formed data. Rolled
 * affixes whose definition no longer exists are dropped, and definitions whose
 * category no longer exists become uncounted, so nothing ever refers to a
 * name the current configuration cannot resolve.
 */
export function normalizeWwmGearTrackerData(raw: unknown): WwmGearTrackerData {
  if (typeof raw !== "object" || raw == null) return emptyWwmGearTrackerData();
  const source = raw as Record<string, unknown>;

  const affixCategories: AffixCategory[] = [];
  if (Array.isArray(source.affixCategories)) {
    for (const entry of source.affixCategories) {
      if (typeof entry !== "object" || entry == null) continue;
      const { id, name, needed } = entry as Record<string, unknown>;
      const trimmed = normalizeName(name);
      if (trimmed === "") continue;
      affixCategories.push({ id: normalizeId(id), name: trimmed, needed: normalizeNeeded(needed) });
    }
  }

  const knownCategoryIds = new Set(affixCategories.map(({ id }) => id));
  const affixPool: AffixDefinition[] = [];
  if (Array.isArray(source.affixPool)) {
    for (const entry of source.affixPool) {
      if (typeof entry !== "object" || entry == null) continue;
      const { id, name, categoryId } = entry as Record<string, unknown>;
      const trimmed = normalizeName(name);
      if (trimmed === "") continue;
      affixPool.push({
        id: normalizeId(id),
        name: trimmed,
        categoryId:
          typeof categoryId === "string" && knownCategoryIds.has(categoryId) ? categoryId : null,
      });
    }
  }

  const knownAffixIds = new Set(affixPool.map(({ id }) => id));
  const gears: Gear[] = [];
  if (Array.isArray(source.gears)) {
    for (const entry of source.gears) {
      if (typeof entry !== "object" || entry == null) continue;
      const gear = entry as Record<string, unknown>;
      const affixes: RolledAffix[] = [];
      if (Array.isArray(gear.affixes)) {
        for (const affix of gear.affixes) {
          if (typeof affix !== "object" || affix == null) continue;
          const { id, definitionId, value } = affix as Record<string, unknown>;
          if (typeof definitionId !== "string" || !knownAffixIds.has(definitionId)) continue;
          affixes.push({
            id: normalizeId(id),
            definitionId,
            value: normalizeName(value),
          });
        }
      }
      const rawCount = typeof gear.affixCount === "number" ? gear.affixCount : Number.NaN;
      gears.push({
        id: normalizeId(gear.id),
        name: normalizeName(gear.name),
        slot: normalizeName(gear.slot),
        affixCount: Number.isFinite(rawCount)
          ? Math.min(99, Math.max(1, Math.round(rawCount)))
          : Math.max(1, affixes.length),
        affixes,
      });
    }
  }
  return { version: 1, affixCategories, affixPool, gears };
}

/** Maps each pool entry to the category it fills, or null when it never counts. */
export function categoryOfAffix({
  affixCategories,
  affixPool,
}: WwmGearTrackerData): ReadonlyMap<string, AffixCategory | null> {
  const byId = new Map(affixCategories.map((category) => [category.id, category]));
  const map = new Map<string, AffixCategory | null>();
  for (const definition of affixPool) {
    map.set(
      definition.id,
      definition.categoryId == null ? null : (byId.get(definition.categoryId) ?? null),
    );
  }
  return map;
}

/** How one category stands across every piece combined. */
export type CategoryFill = {
  category: AffixCategory;
  // Affixes of this category rolled across every piece.
  rolled: number;
  // Rolled affixes that count: capped at the category's quota.
  counted: number;
};

/**
 * The ids of rolled affixes that count toward their category's quota: the
 * first `needed` affixes of each category across all pieces, in piece then
 * roll order. The rest are overfills and render as uncounted.
 */
export function countedAffixIds(data: WwmGearTrackerData): ReadonlySet<string> {
  const categoryOf = categoryOfAffix(data);
  const seen = new Map<string, number>();
  const counted = new Set<string>();
  for (const gear of data.gears) {
    for (const affix of gear.affixes) {
      const category = categoryOf.get(affix.definitionId);
      if (category == null || category.needed <= 0) continue;
      const n = (seen.get(category.id) ?? 0) + 1;
      seen.set(category.id, n);
      if (n <= category.needed) counted.add(affix.id);
    }
  }
  return counted;
}

/** Each category's quota, filled by the affixes rolled across every piece. */
export function overallCategoryFills(data: WwmGearTrackerData): CategoryFill[] {
  const categoryOf = categoryOfAffix(data);
  const rolledBy = new Map<string, number>();
  for (const gear of data.gears) {
    for (const affix of gear.affixes) {
      const category = categoryOf.get(affix.definitionId);
      if (category == null) continue;
      rolledBy.set(category.id, (rolledBy.get(category.id) ?? 0) + 1);
    }
  }
  const fills: CategoryFill[] = [];
  for (const category of data.affixCategories) {
    if (category.needed <= 0) continue;
    const rolled = rolledBy.get(category.id) ?? 0;
    fills.push({ category, rolled, counted: Math.min(rolled, category.needed) });
  }
  return fills;
}

export type GearStats = {
  rolled: number;
  // How many of this piece's rolled affixes count toward the shared quotas.
  counted: number;
};

export function gearStats(gear: Gear, countedIds: ReadonlySet<string>): GearStats {
  let counted = 0;
  for (const affix of gear.affixes) {
    if (countedIds.has(affix.id)) counted++;
  }
  return { rolled: gear.affixes.length, counted };
}

export type OverallStats = {
  pieces: number;
  // Total quota across categories — defined once, shared by every piece.
  required: number;
  // Quota filled by all pieces' rolled affixes together.
  filled: number;
  // Whether every category's quota is met.
  graduated: boolean;
  // How many more counted affixes are needed to graduate.
  remaining: number;
};

export function overallStats(data: WwmGearTrackerData): OverallStats {
  let required = 0;
  let filled = 0;
  for (const { category, counted } of overallCategoryFills(data)) {
    required += category.needed;
    filled += counted;
  }
  return {
    pieces: data.gears.length,
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

/** How many rolled affixes across all pieces use each pool entry. */
export function definitionUsage({ gears }: WwmGearTrackerData): Map<string, number> {
  const usage = new Map<string, number>();
  for (const gear of gears) {
    for (const affix of gear.affixes) {
      usage.set(affix.definitionId, (usage.get(affix.definitionId) ?? 0) + 1);
    }
  }
  return usage;
}
