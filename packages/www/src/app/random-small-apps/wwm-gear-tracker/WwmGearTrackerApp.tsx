import { useEffect, useMemo, useState } from "react";
import { useAuth } from "../../../lib/useAuth";
import { loadSmallAppData, saveSmallAppData } from "../smallAppStorage";
import {
  countedAffixIds,
  definitionUsage,
  emptyWwmGearTrackerData,
  formatRate,
  gearStats,
  newId,
  normalizeWwmGearTrackerData,
  overallCategoryFills,
  overallStats,
  type AffixDefinition,
  type Gear,
  type RolledAffix,
  type WwmGearTrackerData,
} from "./types";

const APP_ID = "wwm-gear-tracker";

const inputCls =
  "rounded border border-gray-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";

const primaryButtonCls =
  "rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300";

const secondaryButtonCls =
  "rounded border border-gray-300 px-4 py-2 text-sm font-bold text-gray-700 transition-colors duration-200 hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";

const dangerButtonCls =
  "rounded border border-red-400 px-3 py-1.5 text-sm font-bold text-red-600 transition-colors duration-200 hover:bg-red-50 dark:border-red-500 dark:text-red-400 dark:hover:bg-red-900/20";

const labelCls = "mb-1 text-gray-600 dark:text-gray-400";

function Card({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col bg-white transition-all duration-300 ease-out p-6 dark:bg-[#242424] max-sm:-mx-6 sm:rounded sm:filter sm:drop-shadow sm:hover:drop-shadow-lg sm:dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] sm:dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
      {children}
    </div>
  );
}

/** A filled share of a whole, as a bar. `0` when there is nothing to measure. */
function share(numerator: number, denominator: number): number {
  if (denominator <= 0) return 0;
  return Math.min(1, numerator / denominator);
}

function ProgressBar({
  numerator,
  denominator,
}: {
  numerator: number;
  denominator: number;
}): React.JSX.Element {
  return (
    <div
      className="flex h-3 w-full overflow-hidden bg-white/10"
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={denominator}
      aria-valuenow={numerator}
    >
      <div
        className="h-full bg-blue-400"
        style={{ width: `${share(numerator, denominator) * 100}%` }}
      />
    </div>
  );
}

/** The hero's headline rate, with a smaller decimal like the in-canada hero. */
function RateFigure({ rate }: { rate: string }): React.JSX.Element {
  const match = /^(\d+)(\.\d+)?%$/.exec(rate);
  if (match == null) {
    return <span className="text-7xl font-bold leading-none tracking-tight">—</span>;
  }
  const [, whole, fraction] = match;
  return (
    <span className="font-bold leading-none tracking-tight tabular-nums">
      <span className="text-7xl">{whole}</span>
      <span className="text-3xl text-slate-300">{`${fraction ?? ""}%`}</span>
    </span>
  );
}

function Hero({ data }: { data: WwmGearTrackerData }): React.JSX.Element {
  const stats = overallStats(data);
  const fills = overallCategoryFills(data);
  return (
    <section className="bg-slate-900 text-white">
      <div className="mx-auto max-w-6xl px-6 pt-12 pb-10">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
          Random Small Apps
        </p>
        <h1 className="mt-2 font-sans text-3xl font-bold tracking-tight text-white">
          WWM Gear Tracker
        </h1>
        <p className="mt-3 max-w-2xl text-sm text-slate-300">
          Track the affixes on your Where Winds Meet gear against quotas you set yourself: define
          categories of affixes and how many of each your gear needs. The quotas are shared across
          every piece — all your pieces&apos; rolled affixes fill them together, and extra affixes
          in one category never make up for a shortfall in another.
        </p>

        <div className="mt-8 flex flex-wrap items-end justify-between gap-x-12 gap-y-6">
          <div>
            <div className="flex flex-wrap items-baseline gap-x-4 gap-y-2">
              <RateFigure rate={formatRate(stats.filled, stats.required)} />
              <span className="text-lg text-slate-400">graduation rate</span>
            </div>
            <p className="mt-4 text-slate-300">
              {stats.required > 0
                ? `${stats.filled} of ${stats.required} required affixes filled · ${stats.pieces} ${stats.pieces === 1 ? "piece" : "pieces"} · ${stats.graduated ? "graduated" : `${stats.remaining} to go`}`
                : "No requirements defined yet"}
            </p>
          </div>
          <div className="flex max-w-xl flex-wrap justify-end gap-2">
            {fills.length === 0 ? (
              <p className="text-sm text-slate-400">
                Set up categories with quotas in Affix Setup to start tracking.
              </p>
            ) : (
              fills.map(({ category, counted, rolled }) => (
                <span
                  key={category.id}
                  className={`rounded border px-2 py-1 text-xs ${
                    counted >= category.needed
                      ? "border-emerald-400/60 bg-emerald-400/10 text-emerald-200"
                      : "border-blue-400/40 bg-blue-400/10 text-blue-200"
                  }`}
                >
                  {`${category.name} · ${counted}/${category.needed} (rolled ${rolled})`}
                </span>
              ))
            )}
          </div>
        </div>
      </div>
      <ProgressBar numerator={stats.filled} denominator={stats.required} />
    </section>
  );
}

type TabId = "gear" | "setup";

const TABS: readonly { id: TabId; label: string }[] = [
  { id: "gear", label: "My Gear" },
  { id: "setup", label: "Affix Setup" },
];

function TabBar({
  tab,
  onSelect,
}: {
  tab: TabId;
  onSelect: (tab: TabId) => void;
}): React.JSX.Element {
  return (
    <div className="flex border-b border-gray-200 dark:border-gray-700" role="tablist">
      {TABS.map(({ id, label }) => {
        const isActive = id === tab;
        const cls = isActive
          ? "border-b-2 border-blue-500 text-blue-500 dark:border-blue-400 dark:text-blue-400"
          : "border-b-2 border-transparent text-gray-600 hover:text-blue-500 dark:text-gray-300 dark:hover:text-blue-400";
        return (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={isActive}
            className={`whitespace-nowrap px-4 py-2 text-sm font-medium transition-colors duration-200 ${cls}`}
            onClick={() => onSelect(id)}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

/** The form for one piece, shared by adding and editing. */
function GearForm({
  initial,
  data,
  pieceNumber,
  onSubmit,
  onCancel,
  onGoToSetup,
}: {
  // The piece being edited, or null when this form adds a new one.
  initial: Gear | null;
  data: WwmGearTrackerData;
  // Fallback name for a piece saved with an empty name.
  pieceNumber: number;
  onSubmit: (gear: Gear) => void;
  onCancel: () => void;
  onGoToSetup: () => void;
}): React.JSX.Element {
  const [name, setName] = useState(initial?.name ?? "");
  const [slot, setSlot] = useState(initial?.slot ?? "");
  const [affixCount, setAffixCount] = useState(initial?.affixCount.toString() ?? "4");
  const [affixes, setAffixes] = useState<RolledAffix[]>(
    initial?.affixes.map((affix) => ({ ...affix })) ?? [],
  );

  const parsed = Number.parseInt(affixCount, 10);
  const slots = Number.isFinite(parsed) ? Math.min(99, Math.max(1, parsed)) : 1;
  const canAddAffix = data.affixPool.length > 0 && affixes.length < slots;

  const addAffix = () => {
    const first = data.affixPool[0];
    if (first == null) return;
    setAffixes((list) => [...list, { id: newId(), definitionId: first.id, value: "" }]);
  };

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      id: initial?.id ?? newId(),
      name: name.trim() === "" ? `Piece ${pieceNumber}` : name.trim(),
      slot: slot.trim(),
      affixCount: slots,
      // More rolled affixes than the (possibly lowered) slot count are dropped.
      affixes: affixes.slice(0, slots).map((affix) => ({ ...affix, value: affix.value.trim() })),
    });
  };

  const poolByCategory = useMemo(() => {
    const grouped = new Map<string, AffixDefinition[]>();
    for (const category of data.affixCategories) grouped.set(category.id, []);
    const uncategorized: AffixDefinition[] = [];
    for (const definition of data.affixPool) {
      if (definition.categoryId == null) uncategorized.push(definition);
      else grouped.get(definition.categoryId)?.push(definition);
    }
    return { grouped, uncategorized };
  }, [data.affixCategories, data.affixPool]);

  return (
    <Card>
      <h3 className="mb-4">{initial == null ? "Add a piece" : "Edit piece"}</h3>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <label className="flex flex-col text-sm">
            <span className={labelCls}>Name</span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={`Defaults to "Piece ${pieceNumber}"`}
              className={inputCls}
            />
          </label>
          <label className="flex flex-col text-sm">
            <span className={labelCls}>Slot (optional)</span>
            <input
              value={slot}
              onChange={(e) => setSlot(e.target.value)}
              placeholder="e.g. Ring"
              className={inputCls}
            />
          </label>
        </div>
        <label className="flex flex-col text-sm">
          <span className={labelCls}>Affix slots</span>
          <input
            type="number"
            min={1}
            max={99}
            value={affixCount}
            onChange={(e) => setAffixCount(e.target.value)}
            className={`${inputCls} w-24`}
          />
        </label>

        <div>
          <span className={`text-sm ${labelCls}`}>Rolled affixes</span>
          {data.affixPool.length === 0 ? (
            <div className="mt-2 flex flex-wrap items-center gap-3">
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Define your affixes first — you pick them from that pool.
              </p>
              <button type="button" onClick={onGoToSetup} className={secondaryButtonCls}>
                Go to Affix Setup
              </button>
            </div>
          ) : (
            <>
              <div className="mt-2 flex flex-col gap-2">
                {affixes.map((affix, index) => (
                  <div key={affix.id} className="flex items-center gap-2">
                    <select
                      value={affix.definitionId}
                      onChange={(e) =>
                        setAffixes((list) =>
                          list.map((a, i) =>
                            i === index ? { ...a, definitionId: e.target.value } : a,
                          ),
                        )
                      }
                      aria-label={`Affix ${index + 1}`}
                      className={`${inputCls} flex-1`}
                    >
                      {data.affixCategories.map((category) => (
                        <AffixOptgroup
                          key={category.id}
                          label={`${category.name} (quota ${category.needed})`}
                          definitions={poolByCategory.grouped.get(category.id) ?? []}
                        />
                      ))}
                      {poolByCategory.uncategorized.length > 0 && (
                        <AffixOptgroup
                          label="Not counted"
                          definitions={poolByCategory.uncategorized}
                        />
                      )}
                    </select>
                    <input
                      value={affix.value}
                      onChange={(e) =>
                        setAffixes((list) =>
                          list.map((a, i) => (i === index ? { ...a, value: e.target.value } : a)),
                        )
                      }
                      placeholder="Value (optional)"
                      aria-label={`Affix ${index + 1} value`}
                      className={`${inputCls} w-40`}
                    />
                    <button
                      type="button"
                      onClick={() => setAffixes((list) => list.filter((_, i) => i !== index))}
                      aria-label={`Remove affix ${index + 1}`}
                      title="Remove this affix"
                      className="px-2 py-1 text-lg text-gray-400 transition-colors duration-200 hover:text-red-500"
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
              <div className="mt-2 flex items-center gap-3">
                <button
                  type="button"
                  onClick={addAffix}
                  disabled={!canAddAffix}
                  className={secondaryButtonCls}
                >
                  Add affix
                </button>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  {`${affixes.length} of ${slots} slots rolled`}
                </span>
              </div>
            </>
          )}
        </div>

        <div className="flex justify-end gap-3">
          <button type="button" onClick={onCancel} className={secondaryButtonCls}>
            Cancel
          </button>
          <button type="submit" className={primaryButtonCls}>
            {initial == null ? "Add piece" : "Save changes"}
          </button>
        </div>
      </form>
    </Card>
  );
}

/** The form's affix picker grouped by the category each affix fills. */
function AffixOptgroup({
  label,
  definitions,
}: {
  label: string;
  definitions: readonly AffixDefinition[];
}): React.JSX.Element {
  if (definitions.length === 0) return <></>;
  return (
    <optgroup label={label}>
      {definitions.map((definition) => (
        <option key={definition.id} value={definition.id}>
          {definition.name}
        </option>
      ))}
    </optgroup>
  );
}

function GearCard({
  gear,
  data,
  countedIds,
  onEdit,
  onDelete,
}: {
  gear: Gear;
  data: WwmGearTrackerData;
  // Which rolled affixes count toward the shared quotas — computed once for
  // every piece together, since a piece alone cannot fill a quota.
  countedIds: ReadonlySet<string>;
  onEdit: () => void;
  onDelete: () => void;
}): React.JSX.Element {
  const stats = gearStats(gear, countedIds);
  const definitionsById = useMemo(
    () => new Map(data.affixPool.map((definition) => [definition.id, definition])),
    [data.affixPool],
  );
  const categoriesById = useMemo(
    () => new Map(data.affixCategories.map((category) => [category.id, category])),
    [data.affixCategories],
  );

  const unrolled = Math.max(0, gear.affixCount - gear.affixes.length);
  // The piece's useful share: how many of its rolled affixes count.
  const rate = share(stats.counted, stats.rolled);
  const badgeCls =
    stats.rolled > 0 && rate >= 1
      ? "bg-emerald-500"
      : stats.counted > 0
        ? "bg-blue-500 dark:bg-blue-400"
        : "bg-gray-400 dark:bg-gray-600";

  return (
    <Card>
      <div className="flex items-start justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-lg font-semibold">{gear.name}</span>
          {gear.slot !== "" && (
            <span className="rounded bg-gray-100 px-2 py-0.5 text-xs text-gray-600 dark:bg-gray-700 dark:text-gray-300">
              {gear.slot}
            </span>
          )}
        </div>
        <span
          className={`rounded px-2 py-1 text-sm font-bold tabular-nums text-white ${badgeCls}`}
          title="Rolled affixes counting toward the shared quotas"
        >
          {`${stats.counted}/${stats.rolled}`}
        </span>
      </div>

      <div className="mt-3 h-2 w-full overflow-hidden rounded bg-gray-200 dark:bg-gray-700">
        <div
          className={`h-full ${stats.rolled > 0 && rate >= 1 ? "bg-emerald-500" : "bg-blue-500 dark:bg-blue-400"}`}
          style={{ width: `${rate * 100}%` }}
        />
      </div>
      <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
        {`${stats.counted} of ${stats.rolled} rolled affixes count`}
        {unrolled > 0 ? ` · +${unrolled} unrolled` : ""}
      </p>

      <div className="mt-4 flex flex-wrap gap-2">
        {gear.affixes.map((affix) => {
          const definition = definitionsById.get(affix.definitionId);
          const isCounted = countedIds.has(affix.id);
          const chipCls = isCounted
            ? "rounded border border-emerald-400 bg-emerald-50 px-2 py-1 text-sm text-emerald-800 dark:border-emerald-500 dark:bg-emerald-900/30 dark:text-emerald-300"
            : "rounded border border-gray-300 bg-gray-50 px-2 py-1 text-sm text-gray-600 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-300";
          const category =
            definition?.categoryId == null
              ? null
              : (categoriesById.get(definition.categoryId) ?? null);
          return (
            <span
              key={affix.id}
              className={chipCls}
              title={
                isCounted
                  ? "Counts toward its category's shared quota"
                  : "Does not count — over quota or uncategorized"
              }
            >
              {`${definition?.name ?? "?"}${category != null ? ` · ${category.name}` : ""}${affix.value !== "" ? ` ${affix.value}` : ""}`}
            </span>
          );
        })}
        {unrolled > 0 && (
          <span className="rounded border border-dashed border-gray-300 px-2 py-1 text-sm text-gray-400 dark:border-gray-600 dark:text-gray-500">
            {`+${unrolled} unrolled`}
          </span>
        )}
      </div>

      <div className="mt-4 flex justify-end gap-2">
        <button type="button" onClick={onEdit} className={secondaryButtonCls}>
          Edit
        </button>
        <button type="button" onClick={onDelete} className={dangerButtonCls}>
          Delete
        </button>
      </div>
    </Card>
  );
}

function GearTab({
  draft,
  updateDraft,
  onGoToSetup,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
  onGoToSetup: () => void;
}): React.JSX.Element {
  const [adding, setAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const editing = draft.gears.find(({ id }) => id === editingId) ?? null;
  // Counted affixes depend on every piece at once, so the set is computed here
  // and shared with each card.
  const countedIds = useMemo(() => countedAffixIds(draft), [draft]);

  const submitNew = (gear: Gear) => {
    updateDraft((data) => {
      data.gears.push(gear);
    });
    setAdding(false);
  };
  const submitEdit = (gear: Gear) => {
    updateDraft((data) => {
      const index = data.gears.findIndex(({ id }) => id === gear.id);
      if (index >= 0) data.gears[index] = gear;
    });
    setEditingId(null);
  };
  const deleteGear = (id: string) => {
    updateDraft((data) => {
      data.gears = data.gears.filter((gear) => gear.id !== id);
    });
    if (editingId === id) setEditingId(null);
  };

  return (
    <div className="flex flex-col gap-4">
      {adding ? (
        <GearForm
          initial={null}
          data={draft}
          pieceNumber={draft.gears.length + 1}
          onSubmit={submitNew}
          onCancel={() => setAdding(false)}
          onGoToSetup={() => {
            setAdding(false);
            onGoToSetup();
          }}
        />
      ) : (
        <div className="flex justify-end">
          <button type="button" onClick={() => setAdding(true)} className={primaryButtonCls}>
            Add piece
          </button>
        </div>
      )}

      {draft.gears.length === 0 && !adding ? (
        <Card>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No pieces yet. Set your category quotas in Affix Setup, then add a piece and enter the
            affixes it rolled — nothing about the game is assumed.
          </p>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          {draft.gears.map((gear) =>
            gear.id === editing?.id ? (
              <div key={gear.id} className="sm:col-span-2">
                <GearForm
                  initial={gear}
                  data={draft}
                  pieceNumber={draft.gears.length + 1}
                  onSubmit={submitEdit}
                  onCancel={() => setEditingId(null)}
                  onGoToSetup={onGoToSetup}
                />
              </div>
            ) : (
              <GearCard
                key={gear.id}
                gear={gear}
                data={draft}
                countedIds={countedIds}
                onEdit={() => setEditingId(gear.id)}
                onDelete={() => deleteGear(gear.id)}
              />
            ),
          )}
        </div>
      )}
    </div>
  );
}

function CategoriesCard({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const [name, setName] = useState("");
  const [needed, setNeeded] = useState("2");
  const [error, setError] = useState<string | null>(null);
  const fills = useMemo(() => overallCategoryFills(draft), [draft]);
  const filledByCategory = useMemo(
    () => new Map(fills.map((fill) => [fill.category.id, fill])),
    [fills],
  );

  const parsedNeeded = Number.parseInt(needed, 10);
  const add = () => {
    const trimmed = name.trim();
    if (trimmed === "") {
      setError("Enter a name.");
      return;
    }
    if (draft.affixCategories.some((category) => category.name === trimmed)) {
      setError("That category already exists.");
      return;
    }
    const quota = Number.isFinite(parsedNeeded) ? Math.min(99, Math.max(0, parsedNeeded)) : 1;
    updateDraft((data) => {
      data.affixCategories.push({ id: newId(), name: trimmed, needed: quota });
    });
    setName("");
    setNeeded("2");
    setError(null);
  };

  return (
    <Card>
      <h3 className="mb-2">Categories &amp; quotas</h3>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        A category is a group of affixes with a quota: how many affixes from that group your gear
        needs in total. The quota is shared — rolled affixes from every piece fill it together, and
        your build graduates when every quota is met. Overfilling one category never fills another.
        Deleting a category keeps its affixes, but they stop counting.
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
        className="flex flex-wrap items-end gap-3"
      >
        <label className="flex flex-col text-sm">
          <span className={labelCls}>Name</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Offense"
            className={inputCls}
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className={labelCls}>Needed</span>
          <input
            type="number"
            min={0}
            max={99}
            value={needed}
            onChange={(e) => setNeeded(e.target.value)}
            className={`${inputCls} w-24`}
          />
        </label>
        <button type="submit" className={primaryButtonCls}>
          Add category
        </button>
      </form>
      {error != null && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}

      {draft.affixCategories.length > 0 && (
        <div className="mt-4 flex flex-col gap-2">
          {draft.affixCategories.map((category) => {
            const fill = filledByCategory.get(category.id);
            return (
              <div key={category.id} className="flex flex-wrap items-center gap-2">
                <input
                  value={category.name}
                  aria-label="Category name"
                  onChange={(e) =>
                    updateDraft((data) => {
                      const entry = data.affixCategories.find(({ id }) => id === category.id);
                      if (entry != null) entry.name = e.target.value;
                    })
                  }
                  className={`${inputCls} flex-1`}
                />
                <label className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                  <span className="sr-only">{`Needed for ${category.name}`}</span>
                  <input
                    type="number"
                    min={0}
                    max={99}
                    value={category.needed}
                    aria-label="Quota"
                    onChange={(e) => {
                      const parsed = Number.parseInt(e.target.value, 10);
                      const quota = Number.isFinite(parsed) ? Math.min(99, Math.max(0, parsed)) : 0;
                      updateDraft((data) => {
                        const entry = data.affixCategories.find(({ id }) => id === category.id);
                        if (entry != null) entry.needed = quota;
                      });
                    }}
                    className={`${inputCls} w-16`}
                  />
                  needed
                </label>
                <span className="w-32 shrink-0 text-right text-xs text-gray-500 dark:text-gray-400">
                  {fill != null
                    ? `${fill.counted}/${category.needed} filled (rolled ${fill.rolled})`
                    : "not required"}
                </span>
                <button
                  type="button"
                  onClick={() =>
                    updateDraft((data) => {
                      data.affixCategories = data.affixCategories.filter(
                        ({ id }) => id !== category.id,
                      );
                      for (const definition of data.affixPool) {
                        if (definition.categoryId === category.id) definition.categoryId = null;
                      }
                    })
                  }
                  aria-label={`Delete ${category.name}`}
                  title="Delete this category"
                  className="px-2 py-1 text-lg text-gray-400 transition-colors duration-200 hover:text-red-500"
                >
                  ✕
                </button>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

function AffixPoolCard({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const [name, setName] = useState("");
  const [categoryId, setCategoryId] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const usage = definitionUsage(draft);

  const add = () => {
    const trimmed = name.trim();
    if (trimmed === "") {
      setError("Enter a name.");
      return;
    }
    if (draft.affixPool.some((definition) => definition.name === trimmed)) {
      setError("That affix already exists.");
      return;
    }
    updateDraft((data) => {
      data.affixPool.push({
        id: newId(),
        name: trimmed,
        categoryId: categoryId === "" ? null : categoryId,
      });
    });
    setName("");
    setError(null);
  };

  return (
    <Card>
      <h3 className="mb-2">Affix pool</h3>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        The affixes you can enter on a piece, using the names the game shows you. File each one into
        a category so it counts toward that category&apos;s quota. Deleting an affix also removes it
        from every piece that rolled it.
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
        className="flex flex-wrap items-end gap-3"
      >
        <label className="flex flex-col text-sm">
          <span className={labelCls}>Name</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Crit Rate"
            className={inputCls}
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className={labelCls}>Category</span>
          <select
            value={categoryId}
            onChange={(e) => setCategoryId(e.target.value)}
            className={inputCls}
          >
            <option value="">Not counted</option>
            {draft.affixCategories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>
        </label>
        <button
          type="submit"
          disabled={draft.affixCategories.length === 0}
          title={draft.affixCategories.length === 0 ? "Add a category first" : undefined}
          className={primaryButtonCls}
        >
          Add affix
        </button>
      </form>
      {error != null && <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>}

      {draft.affixPool.length > 0 && (
        <div className="mt-4 flex flex-col gap-2">
          {draft.affixPool.map((definition) => {
            const count = usage.get(definition.id) ?? 0;
            return (
              <div key={definition.id} className="flex flex-wrap items-center gap-2">
                <input
                  value={definition.name}
                  aria-label="Affix name"
                  onChange={(e) =>
                    updateDraft((data) => {
                      const entry = data.affixPool.find(({ id }) => id === definition.id);
                      if (entry != null) entry.name = e.target.value;
                    })
                  }
                  className={`${inputCls} flex-1`}
                />
                <select
                  value={definition.categoryId ?? ""}
                  aria-label={`Category for ${definition.name}`}
                  onChange={(e) =>
                    updateDraft((data) => {
                      const entry = data.affixPool.find(({ id }) => id === definition.id);
                      if (entry != null) {
                        entry.categoryId = e.target.value === "" ? null : e.target.value;
                      }
                    })
                  }
                  className={`${inputCls} w-40`}
                >
                  <option value="">Not counted</option>
                  {draft.affixCategories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
                <span className="w-24 shrink-0 text-right text-xs text-gray-500 dark:text-gray-400">
                  {count === 1 ? "on 1 piece" : `on ${count} pieces`}
                </span>
                <button
                  type="button"
                  onClick={() =>
                    updateDraft((data) => {
                      data.affixPool = data.affixPool.filter(({ id }) => id !== definition.id);
                      for (const gear of data.gears) {
                        gear.affixes = gear.affixes.filter(
                          ({ definitionId }) => definitionId !== definition.id,
                        );
                      }
                    })
                  }
                  aria-label={`Delete ${definition.name}`}
                  title="Delete this affix everywhere"
                  className="px-2 py-1 text-lg text-gray-400 transition-colors duration-200 hover:text-red-500"
                >
                  ✕
                </button>
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

function SaveBar({
  saving,
  dirty,
  message,
  onSave,
  onDiscard,
}: {
  saving: boolean;
  dirty: boolean;
  message: { kind: "ok" | "error"; text: string } | null;
  onSave: () => void;
  onDiscard: () => void;
}): React.JSX.Element {
  return (
    <Card>
      {message != null && (
        <div
          className={`mb-4 rounded border px-3 py-2 text-sm ${
            message.kind === "ok"
              ? "border-blue-400 bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300"
              : "border-red-400 bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"
          }`}
        >
          {message.text}
        </div>
      )}
      <div className="flex flex-wrap items-center justify-end gap-3">
        {dirty && (
          <span className="mr-auto text-sm text-gray-500 dark:text-gray-400">Unsaved changes</span>
        )}
        {dirty && (
          <button
            type="button"
            onClick={onDiscard}
            disabled={saving}
            className={secondaryButtonCls}
          >
            Discard
          </button>
        )}
        <button
          type="button"
          onClick={onSave}
          disabled={!dirty || saving}
          className={primaryButtonCls}
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </Card>
  );
}

type LoadState = "loading" | "ready" | "error";

export default function WwmGearTrackerApp(): React.JSX.Element {
  const { session } = useAuth();
  const userId = session?.user.id ?? null;

  // Tagged with the user it was loaded for, so a stale row reads as "loading"
  // instead of being reset synchronously when the user changes.
  const [loaded, setLoaded] = useState<{
    userId: string;
    state: Exclude<LoadState, "loading">;
  } | null>(null);
  const loadState: LoadState =
    userId != null && loaded?.userId === userId ? loaded.state : "loading";
  const [reloadNonce, setReloadNonce] = useState(0);
  // The persisted document, and the (possibly edited) working copy.
  const [saved, setSaved] = useState<WwmGearTrackerData | null>(null);
  const [draft, setDraft] = useState<WwmGearTrackerData | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [tab, setTab] = useState<TabId>("gear");

  useEffect(() => {
    if (userId == null) return;
    let cancelled = false;
    loadSmallAppData(userId, APP_ID).then(({ data, error }) => {
      if (cancelled) return;
      if (error != null) {
        setLoaded({ userId, state: "error" });
        setMessage({ kind: "error", text: error });
        return;
      }
      const normalized = normalizeWwmGearTrackerData(data);
      setSaved(normalized);
      setDraft(normalized);
      setLoaded({ userId, state: "ready" });
    });
    return () => {
      cancelled = true;
    };
    // `reloadNonce` is deliberately not read here: the Retry button bumps it to
    // run this effect again.
    // oxlint-disable-next-line exhaustive-effect-dependencies
  }, [userId, reloadNonce]);

  useEffect(() => {
    if (message?.kind !== "ok") return;
    const t = setTimeout(() => setMessage(null), 4000);
    return () => clearTimeout(t);
  }, [message]);

  /** Applies `mutate` to a deep copy of the draft, so edits never alias `saved`. */
  const updateDraft = (mutate: (data: WwmGearTrackerData) => void) => {
    setDraft((prev) => {
      const base = prev ?? saved ?? emptyWwmGearTrackerData();
      const next: WwmGearTrackerData = {
        version: 1,
        affixCategories: base.affixCategories.map((category) => ({ ...category })),
        affixPool: base.affixPool.map((definition) => ({ ...definition })),
        gears: base.gears.map((gear) => ({
          ...gear,
          affixes: gear.affixes.map((affix) => ({ ...affix })),
        })),
      };
      mutate(next);
      return next;
    });
  };

  const dirty = draft != null && saved != null && JSON.stringify(draft) !== JSON.stringify(saved);

  const save = async () => {
    if (userId == null || draft == null || saving) return;
    setSaving(true);
    setMessage(null);
    // Normalize on the way in so what is stored is exactly what loads back.
    const toSave = normalizeWwmGearTrackerData(draft);
    const error = await saveSmallAppData(userId, APP_ID, toSave);
    setSaving(false);
    if (error != null) {
      setMessage({ kind: "error", text: error });
      return;
    }
    setSaved(toSave);
    setDraft(toSave);
    setMessage({ kind: "ok", text: "Saved." });
  };

  if (loadState === "loading") {
    return (
      <div className="mx-auto w-full max-w-6xl px-6 py-12">
        <Card>
          <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
        </Card>
      </div>
    );
  }

  if (loadState === "error" || draft == null || saved == null) {
    return (
      <div className="mx-auto w-full max-w-6xl px-6 py-12">
        <Card>
          <h3 className="mb-2">Could not load your data</h3>
          <p className="mb-4 text-sm text-red-600 dark:text-red-400">
            {message?.text ?? "Unknown error."}
          </p>
          <div className="flex justify-end">
            <button
              type="button"
              onClick={() => {
                setMessage(null);
                setLoaded(null);
                setReloadNonce((n) => n + 1);
              }}
              className={primaryButtonCls}
            >
              Retry
            </button>
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <Hero data={draft} />

      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-10">
        <TabBar tab={tab} onSelect={setTab} />
        {tab === "gear" ? (
          <GearTab draft={draft} updateDraft={updateDraft} onGoToSetup={() => setTab("setup")} />
        ) : (
          <div className="flex flex-col gap-4">
            <CategoriesCard draft={draft} updateDraft={updateDraft} />
            <AffixPoolCard draft={draft} updateDraft={updateDraft} />
          </div>
        )}
        <SaveBar
          saving={saving}
          dirty={dirty}
          message={message}
          onSave={() => {
            void save();
          }}
          onDiscard={() => {
            setDraft(saved);
            setMessage(null);
          }}
        />
      </div>
    </div>
  );
}
