import { useEffect, useMemo, useState } from "react";
import { useAuth } from "../../../lib/useAuth";
import { loadSmallAppData, saveSmallAppData } from "../smallAppStorage";
import {
  AFFIX_GROUPS,
  AFFIX_POSITIONS,
  GEAR_SLOTS,
  MAX_AFFIXES,
  MAX_QUOTA,
  affixFills,
  affixUsage,
  clampQuota,
  countedAffixIds,
  emptyWwmGearTrackerData,
  enforceSingleEquipped,
  equippedGears,
  formatRate,
  gearStats,
  newId,
  normalizeWwmGearTrackerData,
  optimalSet,
  quotaOf,
  setStats,
  type Affix,
  type Gear,
  type GearSlot,
  type Quotas,
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

const mutedCls = "text-sm text-gray-500 dark:text-gray-400";

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
  const worn = equippedGears(data);
  const stats = setStats(worn, data.quotas);
  const fills = affixFills(worn, data.quotas);
  return (
    <section className="bg-slate-900 text-white">
      <div className="mx-auto max-w-6xl px-6 pt-12 pb-10">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
          Random Small Apps
        </p>
        <h1 className="mt-2 font-sans text-3xl font-bold tracking-tight text-white">
          燕云十六声 · 装备词条
        </h1>
        <p className="mt-3 max-w-2xl text-sm text-slate-300">
          设定整套装备每种词条需要几条，记录每件装备调律出的词条。毕业率按当前穿戴的装备计算：
          同一种词条的需求由所有部位共同填满，超出需求的部分不计，也不能弥补其他词条的缺口。
        </p>

        <div className="mt-8 flex flex-wrap items-end justify-between gap-x-12 gap-y-6">
          <div>
            <div className="flex flex-wrap items-baseline gap-x-4 gap-y-2">
              <RateFigure rate={formatRate(stats.filled, stats.required)} />
              <span className="text-lg text-slate-400">毕业率</span>
            </div>
            <p className="mt-4 text-slate-300">
              {stats.required > 0
                ? `${stats.filled}/${stats.required} 条需求已满足 · 穿戴 ${stats.pieces} 件 · ${stats.graduated ? "已毕业" : `还差 ${stats.remaining} 条`}`
                : "尚未设定毕业要求"}
            </p>
          </div>
          <div className="flex max-w-xl flex-wrap justify-end gap-2">
            {fills.length === 0 ? (
              <p className="text-sm text-slate-400">在「毕业要求」中设定每种词条需要几条。</p>
            ) : (
              fills.map(({ affix, needed, counted, rolled }) => (
                <span
                  key={affix}
                  title={`已穿戴共 ${rolled} 条`}
                  className={`rounded border px-2 py-1 text-xs ${
                    counted >= needed
                      ? "border-emerald-400/60 bg-emerald-400/10 text-emerald-200"
                      : "border-blue-400/40 bg-blue-400/10 text-blue-200"
                  }`}
                >
                  {`${affix} ${counted}/${needed}`}
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

type TabId = "gear" | "compare" | "quotas";

const TABS: readonly { id: TabId; label: string }[] = [
  { id: "gear", label: "我的装备" },
  { id: "compare", label: "配装对比" },
  { id: "quotas", label: "毕业要求" },
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

/** A 词条 picker grouped as the game groups them, noting each one's quota. */
function AffixSelect({
  value,
  quotas,
  onChange,
  label,
}: {
  value: Affix;
  quotas: Quotas;
  onChange: (affix: Affix) => void;
  label: string;
}): React.JSX.Element {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as Affix)}
      aria-label={label}
      className={`${inputCls} flex-1`}
    >
      {AFFIX_GROUPS.map((group) => (
        <optgroup key={group.name} label={group.name}>
          {group.affixes.map((affix) => {
            const needed = quotaOf(quotas, affix);
            return (
              <option key={affix} value={affix}>
                {needed > 0 ? `${affix}（需 ${needed}）` : affix}
              </option>
            );
          })}
        </optgroup>
      ))}
    </select>
  );
}

/** The form for one piece, shared by adding and editing. */
function GearForm({
  initial,
  data,
  onSubmit,
  onCancel,
}: {
  // The piece being edited, or null when this form adds a new one.
  initial: Gear | null;
  data: WwmGearTrackerData;
  onSubmit: (gear: Gear) => void;
  onCancel: () => void;
}): React.JSX.Element {
  const [name, setName] = useState(initial?.name ?? "");
  const [slot, setSlot] = useState<GearSlot>(initial?.slot ?? GEAR_SLOTS[0]);
  const [affixCount, setAffixCount] = useState(initial?.affixCount ?? MAX_AFFIXES);
  const [affixes, setAffixes] = useState<RolledAffix[]>(
    initial?.affixes.map((rolled) => ({ ...rolled })) ?? [],
  );

  // Fallback name for a piece saved with an empty name, e.g. "环 2".
  const othersInSlot = data.gears.filter(
    (gear) => gear.slot === slot && gear.id !== initial?.id,
  ).length;
  const fallbackName = `${slot} ${othersInSlot + 1}`;

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      id: initial?.id ?? newId(),
      name: name.trim() === "" ? fallbackName : name.trim(),
      slot,
      // A new piece is worn when its slot has nothing worn yet.
      equipped:
        initial?.equipped ?? !data.gears.some((gear) => gear.equipped && gear.slot === slot),
      affixCount,
      // More rolled 词条 than the (possibly lowered) count are dropped.
      affixes: affixes
        .slice(0, affixCount)
        .map((rolled) => ({ ...rolled, value: rolled.value.trim() })),
    });
  };

  return (
    <Card>
      <h3 className="mb-4">{initial == null ? "添加装备" : "编辑装备"}</h3>
      <form onSubmit={submit} className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-3">
          <label className="flex flex-col text-sm">
            <span className={labelCls}>名称</span>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={`默认「${fallbackName}」`}
              className={inputCls}
            />
          </label>
          <label className="flex flex-col text-sm">
            <span className={labelCls}>部位</span>
            <select
              value={slot}
              onChange={(e) => setSlot(e.target.value as GearSlot)}
              className={inputCls}
            >
              {GEAR_SLOTS.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col text-sm">
            <span className={labelCls}>词条数</span>
            <select
              value={affixCount}
              onChange={(e) => setAffixCount(Number(e.target.value))}
              className={inputCls}
            >
              {AFFIX_POSITIONS.map((position, i) => (
                <option key={position} value={i + 1}>
                  {`${i + 1}（至${position}）`}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div>
          <span className={`text-sm ${labelCls}`}>已调律词条</span>
          <div className="mt-2 flex flex-col gap-2">
            {affixes.slice(0, affixCount).map((rolled, index) => (
              <div key={rolled.id} className="flex items-center gap-2">
                <span className="w-6 shrink-0 text-center text-sm font-bold text-gray-500 dark:text-gray-400">
                  {AFFIX_POSITIONS[index]}
                </span>
                <AffixSelect
                  value={rolled.affix}
                  quotas={data.quotas}
                  label={`第 ${index + 1} 条词条`}
                  onChange={(affix) =>
                    setAffixes((list) => list.map((a, i) => (i === index ? { ...a, affix } : a)))
                  }
                />
                <input
                  value={rolled.value}
                  onChange={(e) =>
                    setAffixes((list) =>
                      list.map((a, i) => (i === index ? { ...a, value: e.target.value } : a)),
                    )
                  }
                  placeholder="数值（可选）"
                  aria-label={`第 ${index + 1} 条词条数值`}
                  className={`${inputCls} w-32`}
                />
                <button
                  type="button"
                  onClick={() => setAffixes((list) => list.filter((_, i) => i !== index))}
                  aria-label={`删除第 ${index + 1} 条词条`}
                  title="删除此词条"
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
              onClick={() =>
                setAffixes((list) => [
                  ...list.slice(0, affixCount),
                  { id: newId(), affix: AFFIX_GROUPS[0].affixes[0], value: "" },
                ])
              }
              disabled={affixes.length >= affixCount}
              className={secondaryButtonCls}
            >
              添加词条
            </button>
            <span className={mutedCls}>
              {`已调律 ${Math.min(affixes.length, affixCount)}/${affixCount} 条`}
            </span>
          </div>
        </div>

        <div className="flex justify-end gap-3">
          <button type="button" onClick={onCancel} className={secondaryButtonCls}>
            取消
          </button>
          <button type="submit" className={primaryButtonCls}>
            {initial == null ? "添加" : "保存修改"}
          </button>
        </div>
      </form>
    </Card>
  );
}

function GearCard({
  gear,
  quotas,
  countedIds,
  onToggleEquipped,
  onEdit,
  onDelete,
}: {
  gear: Gear;
  quotas: Quotas;
  // Which rolled 词条 count toward the shared quotas — computed once for the
  // whole worn set, since a piece alone cannot fill a quota.
  countedIds: ReadonlySet<string>;
  onToggleEquipped: () => void;
  onEdit: () => void;
  onDelete: () => void;
}): React.JSX.Element {
  const stats = gearStats(gear, quotas, countedIds);
  const unrolled = Math.max(0, gear.affixCount - gear.affixes.length);
  // Worn pieces are measured by what counts; spare ones by what could.
  const good = gear.equipped ? stats.counted : stats.useful;
  const rate = share(good, gear.affixCount);
  const full = good >= gear.affixCount;
  const badgeCls = full
    ? "bg-emerald-500"
    : good > 0
      ? "bg-blue-500 dark:bg-blue-400"
      : "bg-gray-400 dark:bg-gray-600";

  return (
    <Card>
      <div className="flex items-start justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-lg font-semibold">{gear.name}</span>
          {gear.equipped && (
            <span className="rounded bg-emerald-100 px-2 py-0.5 text-xs text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300">
              已穿戴
            </span>
          )}
        </div>
        <span
          className={`rounded px-2 py-1 text-sm font-bold tabular-nums text-white ${badgeCls}`}
          title={gear.equipped ? "计入毕业的词条 / 词条数" : "有效词条 / 词条数（未穿戴）"}
        >
          {`${good}/${gear.affixCount}`}
        </span>
      </div>

      <div className="mt-3 h-2 w-full overflow-hidden rounded bg-gray-200 dark:bg-gray-700">
        <div
          className={`h-full ${full ? "bg-emerald-500" : "bg-blue-500 dark:bg-blue-400"}`}
          style={{ width: `${rate * 100}%` }}
        />
      </div>
      <p className={`mt-2 ${mutedCls}`}>
        {gear.equipped
          ? `${stats.counted} 条计入毕业`
          : `${stats.useful} 条有效词条 · 未穿戴，不计入`}
        {unrolled > 0 ? ` · ${unrolled} 条未调律` : ""}
      </p>

      <div className="mt-4 flex flex-wrap gap-2">
        {gear.affixes.map((rolled, index) => {
          const isCounted = countedIds.has(rolled.id);
          const isUseful = quotaOf(quotas, rolled.affix) > 0;
          const chipCls = isCounted
            ? "border-emerald-400 bg-emerald-50 text-emerald-800 dark:border-emerald-500 dark:bg-emerald-900/30 dark:text-emerald-300"
            : !gear.equipped && isUseful
              ? "border-blue-400 bg-blue-50 text-blue-800 dark:border-blue-500 dark:bg-blue-900/30 dark:text-blue-300"
              : "border-gray-300 bg-gray-50 text-gray-600 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-300";
          return (
            <span
              key={rolled.id}
              className={`rounded border px-2 py-1 text-sm ${chipCls}`}
              title={
                isCounted
                  ? "计入毕业要求"
                  : isUseful
                    ? gear.equipped
                      ? "需求已满，不计入"
                      : "有效词条（未穿戴）"
                    : "无效词条"
              }
            >
              {`${AFFIX_POSITIONS[index]} · ${rolled.affix}${rolled.value !== "" ? ` ${rolled.value}` : ""}`}
            </span>
          );
        })}
        {unrolled > 0 && (
          <span className="rounded border border-dashed border-gray-300 px-2 py-1 text-sm text-gray-400 dark:border-gray-600 dark:text-gray-500">
            {`+${unrolled} 未调律`}
          </span>
        )}
      </div>

      <div className="mt-4 flex justify-end gap-2">
        <button type="button" onClick={onToggleEquipped} className={secondaryButtonCls}>
          {gear.equipped ? "卸下" : "穿戴"}
        </button>
        <button type="button" onClick={onEdit} className={secondaryButtonCls}>
          编辑
        </button>
        <button type="button" onClick={onDelete} className={dangerButtonCls}>
          删除
        </button>
      </div>
    </Card>
  );
}

function GearTab({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const [adding, setAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  // Counted 词条 depend on the whole worn set, so the set is computed here and
  // shared with each card.
  const countedIds = useMemo(() => countedAffixIds(equippedGears(draft), draft.quotas), [draft]);

  const submitNew = (gear: Gear) => {
    updateDraft((data) => {
      data.gears.push(gear);
      enforceSingleEquipped(data.gears, gear.id);
    });
    setAdding(false);
  };
  const submitEdit = (gear: Gear) => {
    updateDraft((data) => {
      const index = data.gears.findIndex(({ id }) => id === gear.id);
      if (index >= 0) data.gears[index] = gear;
      enforceSingleEquipped(data.gears, gear.id);
    });
    setEditingId(null);
  };
  const toggleEquipped = (id: string) => {
    updateDraft((data) => {
      const gear = data.gears.find((entry) => entry.id === id);
      if (gear == null) return;
      gear.equipped = !gear.equipped;
      enforceSingleEquipped(data.gears, id);
    });
  };
  const deleteGear = (id: string) => {
    updateDraft((data) => {
      data.gears = data.gears.filter((gear) => gear.id !== id);
    });
    if (editingId === id) setEditingId(null);
  };

  const slotsInUse = GEAR_SLOTS.filter((slot) => draft.gears.some((gear) => gear.slot === slot));

  return (
    <div className="flex flex-col gap-4">
      {adding ? (
        <GearForm
          initial={null}
          data={draft}
          onSubmit={submitNew}
          onCancel={() => setAdding(false)}
        />
      ) : (
        <div className="flex justify-end">
          <button type="button" onClick={() => setAdding(true)} className={primaryButtonCls}>
            添加装备
          </button>
        </div>
      )}

      {draft.gears.length === 0 && !adding ? (
        <Card>
          <p className={mutedCls}>
            还没有装备。先在「毕业要求」中设定每种词条需要几条，再添加装备并填入调律出的词条。
            同一部位可以录入多件，用「配装对比」找出最优穿戴。
          </p>
        </Card>
      ) : (
        slotsInUse.map((slot) => {
          const pieces = draft.gears.filter((gear) => gear.slot === slot);
          return (
            <section key={slot} className="flex flex-col gap-3">
              <h3 className="text-base font-semibold">
                {slot}
                <span className={`ml-2 font-normal ${mutedCls}`}>{`${pieces.length} 件`}</span>
              </h3>
              <div className="grid gap-4 sm:grid-cols-2">
                {pieces.map((gear) =>
                  gear.id === editingId ? (
                    <div key={gear.id} className="sm:col-span-2">
                      <GearForm
                        initial={gear}
                        data={draft}
                        onSubmit={submitEdit}
                        onCancel={() => setEditingId(null)}
                      />
                    </div>
                  ) : (
                    <GearCard
                      key={gear.id}
                      gear={gear}
                      quotas={draft.quotas}
                      countedIds={countedIds}
                      onToggleEquipped={() => toggleEquipped(gear.id)}
                      onEdit={() => setEditingId(gear.id)}
                      onDelete={() => deleteGear(gear.id)}
                    />
                  ),
                )}
              </div>
            </section>
          );
        })
      )}
    </div>
  );
}

function fillCls(counted: number, needed: number): string {
  if (needed > 0 && counted >= needed) return "text-emerald-600 dark:text-emerald-400";
  return "text-gray-700 dark:text-gray-200";
}

function CompareTab({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const optimal = useMemo(() => optimalSet(draft), [draft]);
  const worn = equippedGears(draft);
  const current = setStats(worn, draft.quotas);
  const best = setStats(optimal.gears, draft.quotas);
  const currentFills = new Map(affixFills(worn, draft.quotas).map((fill) => [fill.affix, fill]));
  const bestFills = affixFills(optimal.gears, draft.quotas);
  const optimalIds = new Set(optimal.gears.map(({ id }) => id));
  const sameAsWorn =
    worn.length === optimal.gears.length && worn.every(({ id }) => optimalIds.has(id));
  const gain = best.filled - current.filled;

  if (draft.gears.length === 0) {
    return (
      <Card>
        <p className={mutedCls}>先在「我的装备」中添加装备。</p>
      </Card>
    );
  }
  if (best.required === 0) {
    return (
      <Card>
        <p className={mutedCls}>先在「毕业要求」中设定每种词条需要几条，才能比较配装。</p>
      </Card>
    );
  }

  const wear = () =>
    updateDraft((data) => {
      for (const gear of data.gears) gear.equipped = optimalIds.has(gear.id);
    });

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="grid gap-6 sm:grid-cols-2">
          {[
            { label: "当前穿戴", stats: current },
            { label: "最优配装", stats: best },
          ].map(({ label, stats }) => (
            <div key={label}>
              <p className={mutedCls}>{label}</p>
              <p className="mt-1 text-4xl font-bold tabular-nums">
                {formatRate(stats.filled, stats.required)}
              </p>
              <p className={`mt-1 ${mutedCls}`}>
                {`${stats.filled}/${stats.required} 条需求 · ${stats.pieces} 件 · ${stats.graduated ? "已毕业" : `还差 ${stats.remaining} 条`}`}
              </p>
            </div>
          ))}
        </div>
        <div className="mt-6 flex flex-wrap items-center justify-end gap-3">
          <span className={`mr-auto ${mutedCls}`}>
            {sameAsWorn
              ? "当前穿戴已是最优配装。"
              : gain > 0
                ? `换上最优配装可多满足 ${gain} 条需求。`
                : "最优配装满足的需求与当前相同，但词条分布更好或穿戴更完整。"}
            {optimal.exhaustive ? "" : " 组合过多，结果为搜索上限内找到的最佳配装。"}
          </span>
          <button type="button" onClick={wear} disabled={sameAsWorn} className={primaryButtonCls}>
            穿戴最优配装
          </button>
        </div>
      </Card>

      <Card>
        <h3 className="mb-4">部位</h3>
        <table className="w-full text-left text-sm">
          <thead className="text-gray-500 dark:text-gray-400">
            <tr>
              <th className="py-1 pr-4 font-normal">部位</th>
              <th className="py-1 pr-4 font-normal">当前</th>
              <th className="py-1 font-normal">最优</th>
            </tr>
          </thead>
          <tbody>
            {GEAR_SLOTS.map((slot) => {
              const now = worn.find((gear) => gear.slot === slot);
              const next = optimal.gears.find((gear) => gear.slot === slot);
              if (now == null && next == null) return null;
              const changed = now?.id !== next?.id;
              return (
                <tr key={slot} className="border-t border-gray-200 dark:border-gray-700">
                  <td className="py-2 pr-4">{slot}</td>
                  <td className="py-2 pr-4 text-gray-500 dark:text-gray-400">{now?.name ?? "—"}</td>
                  <td
                    className={`py-2 ${changed ? "font-semibold text-blue-600 dark:text-blue-400" : ""}`}
                  >
                    {`${next?.name ?? "—"}${changed ? "（更换）" : ""}`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>

      <Card>
        <h3 className="mb-4">词条</h3>
        <table className="w-full text-left text-sm">
          <thead className="text-gray-500 dark:text-gray-400">
            <tr>
              <th className="py-1 pr-4 font-normal">词条</th>
              <th className="py-1 pr-4 font-normal">需求</th>
              <th className="py-1 pr-4 font-normal">当前</th>
              <th className="py-1 font-normal">最优</th>
            </tr>
          </thead>
          <tbody>
            {bestFills.map(({ affix, needed, counted, rolled }) => {
              const now = currentFills.get(affix);
              const nowCounted = now?.counted ?? 0;
              const delta = counted - nowCounted;
              return (
                <tr key={affix} className="border-t border-gray-200 dark:border-gray-700">
                  <td className="py-2 pr-4">{affix}</td>
                  <td className="py-2 pr-4 tabular-nums">{needed}</td>
                  <td
                    className={`py-2 pr-4 tabular-nums ${fillCls(nowCounted, needed)}`}
                    title={`共 ${now?.rolled ?? 0} 条`}
                  >
                    {`${nowCounted}/${needed}`}
                  </td>
                  <td
                    className={`py-2 tabular-nums ${fillCls(counted, needed)}`}
                    title={`共 ${rolled} 条`}
                  >
                    {`${counted}/${needed}`}
                    {delta !== 0 && (
                      <span
                        className={`ml-2 text-xs ${delta > 0 ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-400"}`}
                      >
                        {delta > 0 ? `+${delta}` : `${delta}`}
                      </span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>
    </div>
  );
}

function QuotasTab({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const worn = equippedGears(draft);
  const wornUsage = affixUsage(worn);
  const allUsage = affixUsage(draft.gears);

  return (
    <Card>
      <h3 className="mb-2">毕业要求</h3>
      <p className={`mb-4 ${mutedCls}`}>
        整套穿戴装备每种词条需要几条，0
        表示不需要（无效词条）。需求由所有部位共同填满，超出部分不计。
      </p>
      <div className="flex flex-col gap-6">
        {AFFIX_GROUPS.map((group) => (
          <div key={group.name}>
            <h4 className="mb-2 text-sm font-semibold text-gray-700 dark:text-gray-200">
              {group.name}
            </h4>
            <div className="grid gap-x-6 gap-y-2 sm:grid-cols-2">
              {group.affixes.map((affix) => {
                const needed = quotaOf(draft.quotas, affix);
                const rolled = wornUsage.get(affix) ?? 0;
                const owned = allUsage.get(affix) ?? 0;
                return (
                  <div key={affix} className="flex items-center gap-2">
                    <span className="flex-1 text-sm">{affix}</span>
                    <input
                      type="number"
                      min={0}
                      max={MAX_QUOTA}
                      value={needed}
                      aria-label={`${affix}需要几条`}
                      onChange={(e) => {
                        const quota = clampQuota(Number.parseInt(e.target.value, 10));
                        updateDraft((data) => {
                          if (quota > 0) data.quotas[affix] = quota;
                          else delete data.quotas[affix];
                        });
                      }}
                      className={`${inputCls} w-20`}
                    />
                    <span
                      className={`w-28 shrink-0 text-right text-xs ${needed > 0 && rolled >= needed ? "text-emerald-600 dark:text-emerald-400" : "text-gray-500 dark:text-gray-400"}`}
                      title={`所有装备共 ${owned} 条`}
                    >
                      {needed > 0
                        ? `穿戴 ${Math.min(rolled, needed)}/${needed}`
                        : `穿戴 ${rolled} 条`}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
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
        {dirty && <span className={`mr-auto ${mutedCls}`}>有未保存的修改</span>}
        {dirty && (
          <button
            type="button"
            onClick={onDiscard}
            disabled={saving}
            className={secondaryButtonCls}
          >
            放弃修改
          </button>
        )}
        <button
          type="button"
          onClick={onSave}
          disabled={!dirty || saving}
          className={primaryButtonCls}
        >
          {saving ? "保存中…" : "保存"}
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
        version: 2,
        quotas: { ...base.quotas },
        gears: base.gears.map((gear) => ({
          ...gear,
          affixes: gear.affixes.map((rolled) => ({ ...rolled })),
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
    setMessage({ kind: "ok", text: "已保存。" });
  };

  if (loadState === "loading") {
    return (
      <div className="mx-auto w-full max-w-6xl px-6 py-12">
        <Card>
          <div className="text-center text-gray-500 dark:text-gray-400">加载中…</div>
        </Card>
      </div>
    );
  }

  if (loadState === "error" || draft == null || saved == null) {
    return (
      <div className="mx-auto w-full max-w-6xl px-6 py-12">
        <Card>
          <h3 className="mb-2">无法加载数据</h3>
          <p className="mb-4 text-sm text-red-600 dark:text-red-400">
            {message?.text ?? "未知错误。"}
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
              重试
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
          <GearTab draft={draft} updateDraft={updateDraft} />
        ) : tab === "compare" ? (
          <CompareTab draft={draft} updateDraft={updateDraft} />
        ) : (
          <QuotasTab draft={draft} updateDraft={updateDraft} />
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
