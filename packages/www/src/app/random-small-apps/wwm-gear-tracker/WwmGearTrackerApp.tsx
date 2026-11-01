import { useEffect, useMemo, useState } from "react";
import { useAuth } from "../../../lib/useAuth";
import { loadSmallAppData, saveSmallAppData } from "../smallAppStorage";
import {
  AFFIX_GROUPS,
  GEAR_SLOTS,
  MAX_AFFIXES,
  affixFills,
  affixUsage,
  compareToTarget,
  emptyWwmGearTrackerData,
  enforceSingleEquipped,
  equippedGears,
  formatRate,
  matchedAffixIds,
  newId,
  normalizeWwmGearTrackerData,
  setStats,
  slotTargetUsage,
  targetUsage,
  type Affix,
  type Gear,
  type GearSlot,
  type RolledAffix,
  type TargetSet,
  type WwmGearTrackerData,
} from "./types";

const APP_ID = "wwm-gear-tracker";

const inputCls =
  "rounded border border-gray-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";

const primaryButtonCls =
  "rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300";

const secondaryButtonCls =
  "rounded border border-gray-300 px-4 py-2 text-sm font-bold text-gray-700 transition-colors duration-200 hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";

// Buttons sharing one row of a slot card, sized by the grid rather than their text.
const slotButtonCls =
  "whitespace-nowrap rounded border border-gray-300 px-1 py-1.5 text-sm font-bold text-gray-700 transition-colors duration-200 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";

const slotDangerButtonCls =
  "whitespace-nowrap rounded border border-red-400 px-1 py-1.5 text-sm font-bold text-red-600 transition-colors duration-200 hover:bg-red-50 dark:border-red-500 dark:text-red-400 dark:hover:bg-red-900/20";

const labelCls = "mb-1 text-gray-600 dark:text-gray-400";

const mutedCls = "text-sm text-gray-500 dark:text-gray-400";

function Card({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col bg-white transition-all duration-300 ease-out p-6 dark:bg-[#242424] max-sm:-mx-6 sm:rounded sm:filter sm:drop-shadow sm:hover:drop-shadow-lg sm:dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] sm:dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
      {children}
    </div>
  );
}

/** A card inside the slot grid: never bleeds to the screen edge, tighter on phones. */
function SlotCard({
  highlighted = false,
  children,
}: {
  highlighted?: boolean;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <div
      className={`flex min-w-0 flex-col rounded bg-white p-3 filter drop-shadow transition-all duration-300 ease-out hover:drop-shadow-lg sm:p-5 dark:bg-[#242424] dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)] ${
        highlighted ? "ring-1 ring-emerald-400 dark:ring-emerald-500" : ""
      }`}
    >
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

// The hero's 词条 grid is two or three columns wide; padding to a multiple of
// both keeps its last row whole.
const HERO_GRID_MULTIPLE = 6;

function Hero({ data }: { data: WwmGearTrackerData }): React.JSX.Element {
  const stats = setStats(data);
  const fills = affixFills(data);
  const padding = (HERO_GRID_MULTIPLE - (fills.length % HERO_GRID_MULTIPLE)) % HERO_GRID_MULTIPLE;
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
          在「配装对比」中设定每个部位想要的词条，记录每件装备调律出的词条。毕业率按当前穿戴的装备计算：
          一件装备的词条只有在该部位的目标配装中才计入。
        </p>

        <div className="mt-8 flex flex-wrap items-end justify-between gap-x-12 gap-y-6">
          <div>
            <div className="flex flex-wrap items-baseline gap-x-4 gap-y-2">
              <RateFigure rate={formatRate(stats.filled, stats.required)} />
              <span className="text-lg text-slate-400">毕业率</span>
            </div>
            <p className="mt-4 text-slate-300">
              {stats.required > 0
                ? `${stats.filled}/${stats.required} 条目标词条已满足 · 穿戴 ${stats.pieces} 件 · ${stats.graduated ? "已毕业" : `还差 ${stats.remaining} 条`}`
                : "尚未设定目标配装"}
            </p>
          </div>
          {fills.length === 0 ? (
            <p className="text-sm text-slate-400">在「配装对比」中设定每个部位想要的词条。</p>
          ) : (
            <div className="grid w-full max-w-xl grid-cols-2 gap-px overflow-hidden rounded border border-slate-700 bg-slate-700 text-xs sm:grid-cols-3">
              {fills.map(({ affix, needed, counted, rolled }) => (
                <div
                  key={affix}
                  title={`已穿戴共 ${rolled} 条`}
                  className={`flex h-8 items-center justify-between gap-2 px-3 ${
                    counted >= needed
                      ? "bg-emerald-950 text-emerald-200"
                      : "bg-slate-900 text-slate-200"
                  }`}
                >
                  <span className="min-w-0 truncate">{affix}</span>
                  <span className="shrink-0 tabular-nums">{`${counted}/${needed}`}</span>
                </div>
              ))}
              {Array.from({ length: padding }, (_, i) => (
                <div key={`pad-${i}`} className="h-8 bg-slate-900" />
              ))}
            </div>
          )}
        </div>
      </div>
      <ProgressBar numerator={stats.filled} denominator={stats.required} />
    </section>
  );
}

type TabId = "gear" | "compare";

const TABS: readonly { id: TabId; label: string }[] = [
  { id: "gear", label: "我的装备" },
  { id: "compare", label: "配装对比" },
];

function TabBar({
  tab,
  onSelect,
  trailing,
}: {
  tab: TabId;
  onSelect: (tab: TabId) => void;
  // Shown at the far end of the bar, e.g. the autosave status.
  trailing: React.ReactNode;
}): React.JSX.Element {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-gray-200 dark:border-gray-700">
      <div className="flex" role="tablist">
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
      {trailing}
    </div>
  );
}

/** A 词条 picker grouped as the game groups them, noting which ones a slot targets. */
function AffixSelect({
  value,
  wanted,
  onChange,
  label,
  emptyLabel,
  className = "min-w-0 flex-1",
}: {
  // "" picks `emptyLabel`, offered only when one is given.
  value: Affix | "";
  // How many of each 词条 the slot's target asks for, noted on the option.
  wanted?: ReadonlyMap<Affix, number>;
  onChange: (affix: Affix | "") => void;
  label: string;
  emptyLabel?: string;
  className?: string;
}): React.JSX.Element {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as Affix | "")}
      aria-label={label}
      className={`${inputCls} ${className}`}
    >
      {emptyLabel != null && <option value="">{emptyLabel}</option>}
      {AFFIX_GROUPS.map((group) => (
        <optgroup key={group.name} label={group.name}>
          {group.affixes.map((affix) => {
            const n = wanted?.get(affix) ?? 0;
            return (
              <option key={affix} value={affix}>
                {n > 0 ? `${affix}（目标${n > 1 ? ` ×${n}` : ""}）` : affix}
              </option>
            );
          })}
        </optgroup>
      ))}
    </select>
  );
}

// The in-game layout is two 2×2 blocks side by side: 主武器 副武器 over 环 佩 on
// the left, 头盔 胸甲 over 胫甲 腕甲 on the right. GEAR_SLOTS lists the left
// block first, so two columns put the left block on top of the right one; four
// columns place each slot where the game has it.
const SLOT_PLACEMENT: Readonly<Record<GearSlot, string>> = {
  主武器: "lg:col-start-1 lg:row-start-1",
  副武器: "lg:col-start-2 lg:row-start-1",
  环: "lg:col-start-1 lg:row-start-2",
  佩: "lg:col-start-2 lg:row-start-2",
  头盔: "lg:col-start-3 lg:row-start-1",
  胸甲: "lg:col-start-4 lg:row-start-1",
  胫甲: "lg:col-start-3 lg:row-start-2",
  腕甲: "lg:col-start-4 lg:row-start-2",
};

/** Lays one cell per slot out as the game does. */
function SlotGrid({
  renderSlot,
}: {
  renderSlot: (slot: GearSlot) => React.ReactNode;
}): React.JSX.Element {
  return (
    <div className="grid grid-cols-2 gap-3 sm:gap-4 lg:grid-cols-4">
      {GEAR_SLOTS.map((slot) => (
        <div key={slot} className={`flex min-w-0 flex-col gap-3 ${SLOT_PLACEMENT[slot]}`}>
          {renderSlot(slot)}
        </div>
      ))}
    </div>
  );
}

type AffixRowTone = "match" | "spare" | "missing" | "plain" | "empty";

/** One line of an AffixStack. */
type AffixRow = {
  key: string;
  label: string;
  // Right-aligned, e.g. the rolled value.
  detail?: React.ReactNode;
  tone: AffixRowTone;
  title?: string;
};

const ROW_TONE_CLS: Readonly<Record<AffixRowTone, string>> = {
  match: "bg-emerald-50 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300",
  spare: "bg-blue-50 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  missing: "bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-400",
  plain: "text-gray-700 dark:text-gray-200",
  empty: "text-gray-400 dark:text-gray-500",
};

/**
 * 词条 as a stack of equal rows, always MAX_AFFIXES tall, so every piece's card
 * has the same height whatever it rolled.
 */
function AffixStack({ rows }: { rows: readonly AffixRow[] }): React.JSX.Element {
  const padded: AffixRow[] = [...rows];
  while (padded.length < MAX_AFFIXES) {
    padded.push({ key: `pad-${padded.length}`, label: "—", tone: "empty" });
  }
  return (
    <ul className="flex flex-col divide-y divide-gray-200 overflow-hidden rounded border border-gray-200 text-sm dark:divide-gray-700 dark:border-gray-700">
      {padded.map(({ key, label, detail, tone, title }) => (
        <li
          key={key}
          title={title}
          className={`flex h-9 items-center justify-between gap-2 px-3 ${ROW_TONE_CLS[tone]}`}
        >
          <span className="min-w-0 truncate">{label}</span>
          {detail != null && (
            <span className="min-w-0 max-w-[50%] truncate text-right text-xs tabular-nums opacity-80">
              {detail}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

/** A slot's name, with a trailing control such as the add button. */
function SlotHeader({
  slot,
  note,
  trailing,
}: {
  slot: GearSlot;
  note?: string;
  trailing?: React.ReactNode;
}): React.JSX.Element {
  return (
    <div className="flex h-8 items-center justify-between gap-2">
      <h3 className="min-w-0 truncate text-base font-semibold">
        {slot}
        {note != null && <span className={`ml-2 font-normal ${mutedCls}`}>{note}</span>}
      </h3>
      {trailing}
    </div>
  );
}

/** The form for one piece, shared by adding and editing. */
function GearForm({
  initial,
  initialSlot,
  data,
  onSubmit,
  onCancel,
}: {
  // The piece being edited, or null when this form adds a new one.
  initial: Gear | null;
  // The slot a new piece starts in.
  initialSlot: GearSlot;
  data: WwmGearTrackerData;
  onSubmit: (gear: Gear) => void;
  onCancel: () => void;
}): React.JSX.Element {
  const [name, setName] = useState(initial?.name ?? "");
  const [slot, setSlot] = useState<GearSlot>(initial?.slot ?? initialSlot);
  const [affixCount, setAffixCount] = useState(initial?.affixCount ?? MAX_AFFIXES);
  const [affixes, setAffixes] = useState<RolledAffix[]>(
    initial?.affixes.map((rolled) => ({ ...rolled })) ?? [],
  );
  const wanted = slotTargetUsage(data.target, slot);

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
              {Array.from({ length: MAX_AFFIXES }, (_, i) => i + 1).map((count) => (
                <option key={count} value={count}>
                  {count}
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
                <AffixSelect
                  value={rolled.affix}
                  wanted={wanted}
                  label={`第 ${index + 1} 条词条`}
                  onChange={(affix) => {
                    if (affix === "") return;
                    setAffixes((list) => list.map((a, i) => (i === index ? { ...a, affix } : a)));
                  }}
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
  target,
  onToggleEquipped,
  onEdit,
  onDelete,
}: {
  gear: Gear;
  target: TargetSet;
  onToggleEquipped: () => void;
  onEdit: () => void;
  onDelete: () => void;
}): React.JSX.Element {
  // Worn pieces count what matches their slot's target; spare ones show what would.
  const matchedIds = matchedAffixIds(gear, target);
  const good = matchedIds.size;
  const wanted = target[gear.slot]?.length ?? 0;
  const full = wanted > 0 && good >= wanted;
  const badgeCls = full
    ? "bg-emerald-500"
    : good > 0
      ? "bg-blue-500 dark:bg-blue-400"
      : "bg-gray-400 dark:bg-gray-600";

  const rows: AffixRow[] = gear.affixes.map((rolled) => {
    const isMatch = matchedIds.has(rolled.id);
    return {
      key: rolled.id,
      label: rolled.affix,
      detail: rolled.value !== "" ? rolled.value : undefined,
      tone: isMatch ? (gear.equipped ? "match" : "spare") : "plain",
      title: isMatch ? (gear.equipped ? "计入毕业" : "命中目标（未穿戴）") : "目标外词条",
    };
  });
  for (let i = gear.affixes.length; i < gear.affixCount; i++) {
    rows.push({ key: `unrolled-${i}`, label: "未调律", tone: "empty" });
  }

  return (
    <SlotCard>
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="min-w-0 truncate text-lg font-semibold" title={gear.name}>
            {gear.name}
          </span>
          {gear.equipped && (
            <span className="shrink-0 whitespace-nowrap rounded bg-emerald-100 px-1.5 py-0.5 text-xs text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300">
              已穿戴
            </span>
          )}
        </div>
        <span
          className={`shrink-0 rounded px-2 py-1 text-sm font-bold tabular-nums text-white ${badgeCls}`}
          title={
            gear.equipped ? "计入毕业的词条 / 目标词条数" : "命中目标的词条 / 目标词条数（未穿戴）"
          }
        >
          {wanted > 0 ? `${good}/${wanted}` : "—"}
        </span>
      </div>

      <div className="mt-3 h-2 w-full overflow-hidden rounded bg-gray-200 dark:bg-gray-700">
        <div
          className={`h-full ${full ? "bg-emerald-500" : "bg-blue-500 dark:bg-blue-400"}`}
          style={{ width: `${share(good, wanted) * 100}%` }}
        />
      </div>
      <p className={`mt-2 truncate ${mutedCls}`}>
        {wanted === 0
          ? "未设目标配装"
          : gear.equipped
            ? `${good} 条计入毕业`
            : `${good} 条命中目标 · 未穿戴`}
      </p>

      <div className="mt-3">
        <AffixStack rows={rows} />
      </div>

      <div className="mt-4 grid grid-cols-3 gap-2">
        <button type="button" onClick={onToggleEquipped} className={slotButtonCls}>
          {gear.equipped ? "卸下" : "穿戴"}
        </button>
        <button type="button" onClick={onEdit} className={slotButtonCls}>
          编辑
        </button>
        <button type="button" onClick={onDelete} className={slotDangerButtonCls}>
          删除
        </button>
      </div>
    </SlotCard>
  );
}

function GearTab({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  // The slot a piece is being added to, or null when not adding.
  const [addingSlot, setAddingSlot] = useState<GearSlot | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const editing = draft.gears.find(({ id }) => id === editingId) ?? null;

  const submitNew = (gear: Gear) => {
    updateDraft((data) => {
      data.gears.push(gear);
      enforceSingleEquipped(data.gears, gear.id);
    });
    setAddingSlot(null);
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

  const startAdding = (slot: GearSlot) => {
    setEditingId(null);
    setAddingSlot(slot);
  };

  return (
    <div className="flex flex-col gap-4">
      {editing != null ? (
        <GearForm
          key={editing.id}
          initial={editing}
          initialSlot={editing.slot}
          data={draft}
          onSubmit={submitEdit}
          onCancel={() => setEditingId(null)}
        />
      ) : addingSlot != null ? (
        <GearForm
          key={addingSlot}
          initial={null}
          initialSlot={addingSlot}
          data={draft}
          onSubmit={submitNew}
          onCancel={() => setAddingSlot(null)}
        />
      ) : (
        <div className="flex items-center justify-end gap-3">
          {draft.gears.length === 0 && (
            <p className={`mr-auto ${mutedCls}`}>
              还没有装备。先在「配装对比」中设定每个部位想要的词条，再添加装备并填入调律出的词条。
              同一部位可以录入多件。
            </p>
          )}
          <button
            type="button"
            onClick={() => startAdding(GEAR_SLOTS[0])}
            className={primaryButtonCls}
          >
            添加装备
          </button>
        </div>
      )}

      <SlotGrid
        renderSlot={(slot) => {
          // Worn piece first, spares after.
          const pieces = draft.gears
            .filter((gear) => gear.slot === slot)
            .sort((x, y) => Number(y.equipped) - Number(x.equipped));
          return (
            <>
              <SlotHeader
                slot={slot}
                note={pieces.length > 1 ? `${pieces.length} 件` : undefined}
                trailing={
                  <button
                    type="button"
                    onClick={() => startAdding(slot)}
                    aria-label={`添加${slot}`}
                    title={`添加${slot}`}
                    className="px-2 text-lg text-gray-400 transition-colors duration-200 hover:text-blue-500"
                  >
                    ＋
                  </button>
                }
              />
              {pieces.length === 0 ? (
                <button
                  type="button"
                  onClick={() => startAdding(slot)}
                  className="min-h-32 flex-1 rounded border border-dashed border-gray-300 px-3 text-sm text-gray-400 transition-colors duration-200 hover:border-blue-400 hover:text-blue-500 dark:border-gray-600 dark:text-gray-500"
                >
                  未录入
                </button>
              ) : (
                pieces.map((gear) => (
                  <GearCard
                    key={gear.id}
                    gear={gear}
                    target={draft.target}
                    onToggleEquipped={() => toggleEquipped(gear.id)}
                    onEdit={() => {
                      setAddingSlot(null);
                      setEditingId(gear.id);
                    }}
                    onDelete={() => deleteGear(gear.id)}
                  />
                ))
              )}
            </>
          );
        }}
      />
    </div>
  );
}

const TARGET_ROWS = Array.from({ length: MAX_AFFIXES }, (_, i) => i);

/** Picks the target 词条 for one slot: up to MAX_AFFIXES, order irrelevant. */
function TargetSlotEditor({
  slot,
  affixes,
  onChange,
}: {
  slot: GearSlot;
  affixes: readonly Affix[];
  onChange: (affixes: Affix[]) => void;
}): React.JSX.Element {
  return (
    <SlotCard>
      <div className="flex flex-col gap-2">
        {TARGET_ROWS.map((row) => (
          <AffixSelect
            key={row}
            value={affixes[row] ?? ""}
            emptyLabel="（空）"
            label={`${slot}目标词条 ${row + 1}`}
            className="w-full min-w-0"
            onChange={(affix) => {
              const next: (Affix | "")[] = TARGET_ROWS.map((i) => affixes[i] ?? "");
              next[row] = affix;
              onChange(next.filter((entry): entry is Affix => entry !== ""));
            }}
          />
        ))}
      </div>
    </SlotCard>
  );
}

function CompareTab({
  draft,
  updateDraft,
}: {
  draft: WwmGearTrackerData;
  updateDraft: (mutate: (data: WwmGearTrackerData) => void) => void;
}): React.JSX.Element {
  const [editing, setEditing] = useState(false);
  const comparisons = compareToTarget(draft);
  const bySlot = new Map(comparisons.map((comparison) => [comparison.slot, comparison]));
  const stats = setStats(draft);
  const hasTarget = stats.required > 0;

  const wantedBy = targetUsage(draft.target);
  const wornBy = affixUsage(equippedGears(draft));
  const countedBy = new Map(affixFills(draft).map(({ affix, counted }) => [affix, counted]));
  // Every 词条 the target asks for or the worn set has, in the game's order.
  const affixRows = AFFIX_GROUPS.flatMap(({ affixes }) => affixes).filter(
    (affix) => wantedBy.has(affix) || wornBy.has(affix),
  );

  const setTargetSlot = (slot: GearSlot, affixes: Affix[]) =>
    updateDraft((data) => {
      if (affixes.length > 0) data.target[slot] = affixes;
      else delete data.target[slot];
    });
  const copyWorn = () =>
    updateDraft((data) => {
      data.target = {};
      for (const gear of equippedGears(data)) {
        if (gear.affixes.length > 0)
          data.target[gear.slot] = gear.affixes.map(({ affix }) => affix);
      }
    });

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <p className={mutedCls}>当前穿戴 vs 目标配装</p>
            <p className="mt-1 text-4xl font-bold tabular-nums">
              {formatRate(stats.filled, stats.required)}
            </p>
            <p className={`mt-1 ${mutedCls}`}>
              {hasTarget
                ? `${stats.filled}/${stats.required} 条目标词条已拥有 · ${stats.graduated ? "已毕业" : `还差 ${stats.remaining} 条`}`
                : "还没有目标配装。设定每个部位想要的词条，或以当前穿戴为起点。"}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {editing && (
              <button type="button" onClick={copyWorn} className={secondaryButtonCls}>
                以当前穿戴为目标
              </button>
            )}
            <button
              type="button"
              onClick={() => setEditing((value) => !value)}
              className={primaryButtonCls}
            >
              {editing ? "完成" : "编辑目标配装"}
            </button>
          </div>
        </div>
      </Card>

      <SlotGrid
        renderSlot={(slot) => {
          const comparison = bySlot.get(slot);
          if (comparison == null) return null;
          const { worn, target, matched, missing, extra } = comparison;
          if (editing) {
            return (
              <>
                <SlotHeader slot={slot} />
                <TargetSlotEditor
                  slot={slot}
                  affixes={target}
                  onChange={(affixes) => setTargetSlot(slot, affixes)}
                />
              </>
            );
          }
          // matched + missing is the target and matched + extra is what is worn,
          // so pairing each missing 词条 with an extra one fits in MAX_AFFIXES rows.
          const rows: AffixRow[] = [
            ...matched.map((affix, i): AffixRow => ({
              key: `matched-${i}`,
              label: affix,
              detail: "✓",
              tone: "match",
              title: "已拥有",
            })),
            ...Array.from({ length: Math.max(missing.length, extra.length) }, (_, i): AffixRow => {
              const want = missing[i];
              const have = extra[i];
              if (want == null) {
                return {
                  key: `diff-${i}`,
                  label: have ?? "",
                  detail: "目标外",
                  tone: "plain",
                  title: "目标中没有",
                };
              }
              return {
                key: `diff-${i}`,
                label: `缺 ${want}`,
                detail: have != null ? <span className="line-through">{have}</span> : undefined,
                tone: "missing",
                title: have != null ? `目标 ${want}，现为 ${have}` : `缺少 ${want}`,
              };
            }),
          ];
          const done = target.length > 0 && missing.length === 0;
          return (
            <>
              <SlotHeader slot={slot} />
              <SlotCard highlighted={done}>
                <div className="flex items-center justify-between gap-2">
                  <span className="min-w-0 truncate text-lg font-semibold">
                    {worn?.name ?? "未穿戴"}
                  </span>
                  <span className={`shrink-0 tabular-nums ${mutedCls}`}>
                    {target.length > 0 ? `${matched.length}/${target.length}` : "未设目标"}
                  </span>
                </div>
                <div className="mt-3">
                  <AffixStack rows={rows} />
                </div>
              </SlotCard>
            </>
          );
        }}
      />

      {!editing && affixRows.length > 0 && (
        <Card>
          <h3 className="mb-4">词条</h3>
          <table className="w-full text-left text-sm">
            <thead className="text-gray-500 dark:text-gray-400">
              <tr>
                <th className="py-1 pr-4 font-normal">词条</th>
                <th className="py-1 pr-4 font-normal">目标</th>
                <th className="py-1 pr-4 font-normal" title="穿戴在目标要求的部位上">
                  命中
                </th>
                <th className="py-1 font-normal" title="穿戴装备上共有几条，不论部位">
                  穿戴
                </th>
              </tr>
            </thead>
            <tbody>
              {affixRows.map((affix) => {
                const wanted = wantedBy.get(affix) ?? 0;
                const counted = countedBy.get(affix) ?? 0;
                const have = wornBy.get(affix) ?? 0;
                return (
                  <tr key={affix} className="border-t border-gray-200 dark:border-gray-700">
                    <td className="py-2 pr-4">{affix}</td>
                    <td className="py-2 pr-4 tabular-nums">{wanted}</td>
                    <td
                      className={`py-2 pr-4 tabular-nums ${
                        wanted === 0
                          ? "text-gray-500 dark:text-gray-400"
                          : counted >= wanted
                            ? "text-emerald-600 dark:text-emerald-400"
                            : "text-red-600 dark:text-red-400"
                      }`}
                    >
                      {counted}
                    </td>
                    <td className="py-2 tabular-nums text-gray-500 dark:text-gray-400">{have}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}

function SaveStatus({
  saving,
  dirty,
  error,
  onRetry,
}: {
  saving: boolean;
  dirty: boolean;
  // Why the last save failed, or null.
  error: string | null;
  onRetry: () => void;
}): React.JSX.Element {
  if (saving) return <span className={mutedCls}>保存中…</span>;
  if (error != null) {
    return (
      <span className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
        {`保存失败：${error}`}
        <button
          type="button"
          onClick={onRetry}
          className="font-bold underline transition-colors duration-200 hover:text-red-500"
        >
          重试
        </button>
      </span>
    );
  }
  return <span className={mutedCls}>{dirty ? "即将保存…" : "已自动保存"}</span>;
}

type LoadState = "loading" | "ready" | "error";

// How long edits must pause before they are saved.
const AUTOSAVE_DELAY_MS = 800;

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
  const [loadError, setLoadError] = useState<string | null>(null);
  // The last failed save: its error, and the document it tried to store, which
  // is not retried automatically until the draft changes or 重试 is pressed.
  const [saveFailure, setSaveFailure] = useState<{ error: string; json: string } | null>(null);
  const [tab, setTab] = useState<TabId>("gear");

  useEffect(() => {
    if (userId == null) return;
    let cancelled = false;
    loadSmallAppData(userId, APP_ID).then(({ data, error }) => {
      if (cancelled) return;
      if (error != null) {
        setLoaded({ userId, state: "error" });
        setLoadError(error);
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

  // What the draft stores as, normalized so it is exactly what loads back —
  // comparing that, not the raw draft, keeps a save from re-triggering itself.
  const pending = useMemo(
    () => (draft == null ? null : normalizeWwmGearTrackerData(draft)),
    [draft],
  );
  const pendingJson = pending == null ? null : JSON.stringify(pending);
  const dirty = pendingJson != null && saved != null && pendingJson !== JSON.stringify(saved);

  useEffect(() => {
    if (userId == null || pending == null || pendingJson == null) return;
    if (!dirty || saving || pendingJson === saveFailure?.json) return;
    const t = setTimeout(() => {
      setSaving(true);
      saveSmallAppData(userId, APP_ID, pending).then((error) => {
        setSaving(false);
        if (error != null) {
          setSaveFailure({ error, json: pendingJson });
          return;
        }
        setSaveFailure(null);
        // Only `saved` moves: edits made while this save ran stay in the draft
        // and are saved next.
        setSaved(pending);
      });
    }, AUTOSAVE_DELAY_MS);
    return () => clearTimeout(t);
  }, [userId, pending, pendingJson, dirty, saving, saveFailure]);

  // Leaving with edits not yet stored would lose them.
  useEffect(() => {
    if (!dirty && !saving) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirty, saving]);

  /** Applies `mutate` to a deep copy of the draft, so edits never alias `saved`. */
  const updateDraft = (mutate: (data: WwmGearTrackerData) => void) => {
    setDraft((prev) => {
      const base = prev ?? saved ?? emptyWwmGearTrackerData();
      const next: WwmGearTrackerData = {
        version: 3,
        target: Object.fromEntries(
          Object.entries(base.target).map(([slot, affixes]) => [slot, [...affixes]]),
        ) as TargetSet,
        gears: base.gears.map((gear) => ({
          ...gear,
          affixes: gear.affixes.map((rolled) => ({ ...rolled })),
        })),
      };
      mutate(next);
      return next;
    });
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
          <p className="mb-4 text-sm text-red-600 dark:text-red-400">{loadError ?? "未知错误。"}</p>
          <div className="flex justify-end">
            <button
              type="button"
              onClick={() => {
                setLoadError(null);
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
        <TabBar
          tab={tab}
          onSelect={setTab}
          trailing={
            <SaveStatus
              saving={saving}
              dirty={dirty}
              error={saveFailure?.error ?? null}
              onRetry={() => setSaveFailure(null)}
            />
          }
        />
        {tab === "gear" ? (
          <GearTab draft={draft} updateDraft={updateDraft} />
        ) : (
          <CompareTab draft={draft} updateDraft={updateDraft} />
        )}
      </div>
    </div>
  );
}
