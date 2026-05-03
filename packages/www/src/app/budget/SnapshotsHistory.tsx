"use client";

import { useEffect, useState } from "react";
import { getSupabase } from "../../lib/supabase";
import BulkAddPaste from "./BulkAddPaste";
import BulkBar, { BulkEditField, HeaderCheckbox, RowCheckbox } from "./BulkBar";
import type { InvestmentSnapshot } from "./types";
import { useBulkSelection } from "./useBulkSelection";
import {
  formatCAD,
  friendlyMutationError,
  isoToLocalInputValue,
  localInputValueToISO,
  nowLocalInputValue,
  parseNonNegFloat,
  parsePosFloat,
  snapshotCadValue,
  splitPastedRows,
} from "./utils";

type Props = {
  userId: string | null;
  investmentId: string;
  snapshots: ReadonlyArray<InvestmentSnapshot>;
  loading: boolean;
};

type Draft = {
  total_cost: string;
  total_market_value: string;
  exchange_rate: string;
  recorded_at: string;
};

const emptyDraft = (): Draft => ({
  total_cost: "",
  total_market_value: "",
  exchange_rate: "1",
  recorded_at: nowLocalInputValue(),
});

type BulkEdit = {
  setTotalCost: boolean;
  total_cost: string;
  setTotalMarketValue: boolean;
  total_market_value: string;
  setExchangeRate: boolean;
  exchange_rate: string;
  setRecordedAt: boolean;
  recorded_at: string;
};

const emptyBulkEdit = (): BulkEdit => ({
  setTotalCost: false,
  total_cost: "",
  setTotalMarketValue: false,
  total_market_value: "",
  setExchangeRate: false,
  exchange_rate: "1",
  setRecordedAt: false,
  recorded_at: nowLocalInputValue(),
});

export default function SnapshotsHistory({
  userId,
  investmentId,
  snapshots,
  loading,
}: Props): React.JSX.Element {
  const [showAddForm, setShowAddForm] = useState(false);
  const [addMode, setAddMode] = useState<"single" | "bulk">("single");
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [error, setError] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<Draft>(emptyDraft);
  const [bulkEdit, setBulkEdit] = useState<BulkEdit>(emptyBulkEdit);

  useEffect(() => {
    if (error == null) return;
    const t = setTimeout(() => setError(null), 5000);
    return () => clearTimeout(t);
  }, [error]);

  const validate = (d: Draft): string | null => {
    const cost = parseNonNegFloat(d.total_cost);
    const value = parseNonNegFloat(d.total_market_value);
    const rate = parsePosFloat(d.exchange_rate);
    if (cost == null) return "Total cost must be ≥ 0.";
    if (value == null) return "Total market value must be ≥ 0.";
    if (rate == null) return "Exchange rate must be > 0.";
    if (d.recorded_at === "") return "Recorded at is required.";
    return null;
  };

  const sorted = snapshots
    .slice()
    .sort((a, b) => new Date(b.recorded_at).getTime() - new Date(a.recorded_at).getTime());

  const sel = useBulkSelection(sorted);

  const addSnapshot = async () => {
    if (userId == null) return;
    const err = validate(draft);
    if (err != null) {
      setError(err);
      return;
    }
    setError(null);
    const { error: insErr } = await getSupabase()
      .from("investment_snapshots")
      .insert({
        user_id: userId,
        investment_id: investmentId,
        total_cost: Number(draft.total_cost.replace(/,/g, "")),
        total_market_value: Number(draft.total_market_value.replace(/,/g, "")),
        exchange_rate: Number(draft.exchange_rate.replace(/,/g, "")),
        recorded_at: localInputValueToISO(draft.recorded_at),
      });
    if (insErr != null) {
      setError(friendlyMutationError(insErr));
      return;
    }
    setDraft(emptyDraft());
    setShowAddForm(false);
  };

  const startEdit = (s: InvestmentSnapshot) => {
    setEditingId(s.id);
    setEditDraft({
      total_cost: String(s.total_cost),
      total_market_value: String(s.total_market_value),
      exchange_rate: String(s.exchange_rate),
      recorded_at: isoToLocalInputValue(s.recorded_at),
    });
  };

  const saveEdit = async () => {
    if (editingId == null) return;
    const err = validate(editDraft);
    if (err != null) {
      setError(err);
      return;
    }
    const { error: updErr } = await getSupabase()
      .from("investment_snapshots")
      .update({
        total_cost: Number(editDraft.total_cost.replace(/,/g, "")),
        total_market_value: Number(editDraft.total_market_value.replace(/,/g, "")),
        exchange_rate: Number(editDraft.exchange_rate.replace(/,/g, "")),
        recorded_at: localInputValueToISO(editDraft.recorded_at),
      })
      .eq("id", editingId);
    if (updErr != null) {
      setError(friendlyMutationError(updErr));
      return;
    }
    setEditingId(null);
  };

  const deleteSnapshot = async (id: string) => {
    if (!window.confirm("Delete this snapshot?")) return;
    const { error: delErr } = await getSupabase()
      .from("investment_snapshots")
      .delete()
      .eq("id", id);
    if (delErr != null) setError(friendlyMutationError(delErr));
  };

  const bulkDelete = async () => {
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;
    const { error: err } = await getSupabase().from("investment_snapshots").delete().in("id", ids);
    if (err != null) {
      setError(friendlyMutationError(err));
      return;
    }
    sel.clear();
  };

  const bulkApply = async () => {
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;
    const patch: Record<string, number | string> = {};
    if (bulkEdit.setTotalCost) {
      const c = parseNonNegFloat(bulkEdit.total_cost);
      if (c == null) {
        setError("Total cost must be ≥ 0.");
        return;
      }
      patch.total_cost = c;
    }
    if (bulkEdit.setTotalMarketValue) {
      const v = parseNonNegFloat(bulkEdit.total_market_value);
      if (v == null) {
        setError("Total market value must be ≥ 0.");
        return;
      }
      patch.total_market_value = v;
    }
    if (bulkEdit.setExchangeRate) {
      const r = parsePosFloat(bulkEdit.exchange_rate);
      if (r == null) {
        setError("Exchange rate must be > 0.");
        return;
      }
      patch.exchange_rate = r;
    }
    if (bulkEdit.setRecordedAt) {
      if (bulkEdit.recorded_at === "") {
        setError("Recorded at is required.");
        return;
      }
      patch.recorded_at = localInputValueToISO(bulkEdit.recorded_at);
    }
    if (Object.keys(patch).length === 0) {
      setError("Tick at least one field to apply.");
      return;
    }
    const { error: err } = await getSupabase()
      .from("investment_snapshots")
      .update(patch)
      .in("id", ids);
    if (err != null) {
      setError(friendlyMutationError(err));
      return;
    }
    setBulkEdit(emptyBulkEdit());
  };

  const bulkInsert = async (text: string) => {
    if (userId == null) return { ok: false as const, message: "Not signed in." };
    const lines = splitPastedRows(text);
    type Payload = {
      user_id: string;
      investment_id: string;
      total_cost: number;
      total_market_value: number;
      exchange_rate: number;
      recorded_at: string;
    };
    const payloads: Payload[] = [];
    for (let i = 0; i < lines.length; i++) {
      const cells = lines[i] ?? [];
      const [recordedStr = "", costStr = "", mvStr = "", fxStr = ""] = cells;
      const cost = parseNonNegFloat(costStr);
      const mv = parseNonNegFloat(mvStr);
      const fx = parsePosFloat(fxStr);
      if (recordedStr === "" || cost == null || mv == null || fx == null) {
        return {
          ok: false as const,
          message: `Line ${i + 1}: need recorded_at, total_cost (≥ 0), total_market_value (≥ 0), exchange_rate (> 0).`,
        };
      }
      const parsed = new Date(recordedStr);
      if (Number.isNaN(parsed.getTime())) {
        return { ok: false as const, message: `Line ${i + 1}: invalid recorded_at.` };
      }
      payloads.push({
        user_id: userId,
        investment_id: investmentId,
        total_cost: cost,
        total_market_value: mv,
        exchange_rate: fx,
        recorded_at: parsed.toISOString(),
      });
    }
    const { error: err } = await getSupabase().from("investment_snapshots").insert(payloads);
    if (err != null) return { ok: false as const, message: friendlyMutationError(err) };
    return { ok: true as const, insertedCount: payloads.length };
  };

  return (
    <div className="px-2 py-3">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="m-0 text-sm font-medium text-gray-700 dark:text-gray-200">
          Snapshot history
        </h4>
        <button
          type="button"
          onClick={() => {
            setShowAddForm((v) => !v);
            setError(null);
          }}
          className="text-sm text-blue-500 hover:underline dark:text-blue-400"
        >
          {showAddForm ? "Cancel" : "Add backfill snapshot"}
        </button>
      </div>

      {error && (
        <div className="mb-2 rounded border border-red-400 bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
          {error}
        </div>
      )}

      {showAddForm && (
        <div className="mb-3 rounded border border-gray-200 p-3 dark:border-gray-700">
          <div className="mb-2 flex justify-end gap-1 text-xs">
            <ModeButton active={addMode === "single"} onClick={() => setAddMode("single")}>
              Single
            </ModeButton>
            <ModeButton active={addMode === "bulk"} onClick={() => setAddMode("bulk")}>
              Bulk paste
            </ModeButton>
          </div>
          {addMode === "single" ? (
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-5">
              <SInput
                label="Total cost"
                type="text"
                inputMode="decimal"
                value={draft.total_cost}
                onChange={(v) => setDraft({ ...draft, total_cost: v })}
              />
              <SInput
                label="Market value"
                type="text"
                inputMode="decimal"
                value={draft.total_market_value}
                onChange={(v) => setDraft({ ...draft, total_market_value: v })}
              />
              <SInput
                label="FX → CAD"
                type="text"
                inputMode="decimal"
                value={draft.exchange_rate}
                onChange={(v) => setDraft({ ...draft, exchange_rate: v })}
              />
              <SInput
                label="Recorded at"
                type="datetime-local"
                value={draft.recorded_at}
                onChange={(v) => setDraft({ ...draft, recorded_at: v })}
              />
              <div className="flex items-end">
                <button
                  type="button"
                  onClick={addSnapshot}
                  className="rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white hover:bg-blue-600 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
                >
                  Save
                </button>
              </div>
            </div>
          ) : (
            <BulkAddPaste
              placeholder={"2026-03-01T16:00\t10000\t10500\t1\n2026-04-01T16:00\t10000\t11250\t1"}
              helpText={
                <span>
                  One row per line: <code>recorded_at</code>, <code>total_cost</code>,{" "}
                  <code>total_market_value</code>, <code>exchange_rate</code>. Tab- or
                  comma-separated.
                </span>
              }
              onSubmit={bulkInsert}
            />
          )}
        </div>
      )}

      <BulkBar
        count={sel.count}
        itemNoun="snapshot"
        onDelete={bulkDelete}
        onClear={sel.clear}
        onApplyEdit={bulkApply}
        editForm={
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-4">
            <BulkEditField
              label="Total cost"
              set={bulkEdit.setTotalCost}
              onSet={(v) => setBulkEdit({ ...bulkEdit, setTotalCost: v })}
            >
              <input
                type="text"
                inputMode="decimal"
                value={bulkEdit.total_cost}
                onChange={(e) => setBulkEdit({ ...bulkEdit, total_cost: e.target.value })}
                className={inputCls()}
              />
            </BulkEditField>
            <BulkEditField
              label="Market value"
              set={bulkEdit.setTotalMarketValue}
              onSet={(v) => setBulkEdit({ ...bulkEdit, setTotalMarketValue: v })}
            >
              <input
                type="text"
                inputMode="decimal"
                value={bulkEdit.total_market_value}
                onChange={(e) => setBulkEdit({ ...bulkEdit, total_market_value: e.target.value })}
                className={inputCls()}
              />
            </BulkEditField>
            <BulkEditField
              label="FX → CAD"
              set={bulkEdit.setExchangeRate}
              onSet={(v) => setBulkEdit({ ...bulkEdit, setExchangeRate: v })}
            >
              <input
                type="text"
                inputMode="decimal"
                step="0.0001"
                value={bulkEdit.exchange_rate}
                onChange={(e) => setBulkEdit({ ...bulkEdit, exchange_rate: e.target.value })}
                className={inputCls()}
              />
            </BulkEditField>
            <BulkEditField
              label="Recorded at"
              set={bulkEdit.setRecordedAt}
              onSet={(v) => setBulkEdit({ ...bulkEdit, setRecordedAt: v })}
            >
              <input
                type="datetime-local"
                value={bulkEdit.recorded_at}
                onChange={(e) => setBulkEdit({ ...bulkEdit, recorded_at: e.target.value })}
                className={inputCls()}
              />
            </BulkEditField>
          </div>
        }
      />

      {loading && <div className="text-sm text-gray-500 dark:text-gray-400">Loading…</div>}
      {!loading && sorted.length === 0 && (
        <div className="text-sm text-gray-500 dark:text-gray-400">No snapshots yet.</div>
      )}
      {!loading && sorted.length > 0 && (
        <div className="overflow-x-auto">
          <table className="min-w-full text-xs">
            <thead>
              <tr className="border-b border-gray-200 text-left dark:border-gray-700">
                <Th className="w-6">
                  <HeaderCheckbox
                    allSelected={sel.allSelected}
                    someSelected={sel.someSelected}
                    onToggle={sel.toggleAll}
                  />
                </Th>
                <Th>Recorded at</Th>
                <Th className="text-right">Cost</Th>
                <Th className="text-right">Market value</Th>
                <Th className="text-right">FX</Th>
                <Th className="text-right">Value (CAD)</Th>
                <Th />
              </tr>
            </thead>
            <tbody>
              {sorted.map((s) => {
                const editing = editingId === s.id;
                return (
                  <tr key={s.id} className="border-b border-gray-100 dark:border-gray-800">
                    <Td className="w-6">
                      <RowCheckbox
                        checked={sel.isSelected(s.id)}
                        onToggle={() => sel.toggle(s.id)}
                      />
                    </Td>
                    {editing ? (
                      <>
                        <Td>
                          <input
                            type="datetime-local"
                            value={editDraft.recorded_at}
                            onChange={(e) =>
                              setEditDraft({ ...editDraft, recorded_at: e.target.value })
                            }
                            className={inputCls()}
                          />
                        </Td>
                        <Td className="text-right">
                          <input
                            type="text"
                            inputMode="decimal"
                            value={editDraft.total_cost}
                            onChange={(e) =>
                              setEditDraft({ ...editDraft, total_cost: e.target.value })
                            }
                            className={`${inputCls()} text-right`}
                          />
                        </Td>
                        <Td className="text-right">
                          <input
                            type="text"
                            inputMode="decimal"
                            value={editDraft.total_market_value}
                            onChange={(e) =>
                              setEditDraft({ ...editDraft, total_market_value: e.target.value })
                            }
                            className={`${inputCls()} text-right`}
                          />
                        </Td>
                        <Td className="text-right">
                          <input
                            type="text"
                            inputMode="decimal"
                            step="0.0001"
                            value={editDraft.exchange_rate}
                            onChange={(e) =>
                              setEditDraft({ ...editDraft, exchange_rate: e.target.value })
                            }
                            className={`${inputCls()} text-right`}
                          />
                        </Td>
                        <Td className="text-right">—</Td>
                        <Td className="text-right whitespace-nowrap">
                          <button
                            type="button"
                            onClick={saveEdit}
                            className="mr-2 text-blue-500 hover:underline dark:text-blue-400"
                          >
                            Save
                          </button>
                          <button
                            type="button"
                            onClick={() => setEditingId(null)}
                            className="text-gray-500 hover:underline dark:text-gray-400"
                          >
                            Cancel
                          </button>
                        </Td>
                      </>
                    ) : (
                      <>
                        <Td>{new Date(s.recorded_at).toLocaleString()}</Td>
                        <Td className="text-right">{s.total_cost.toLocaleString()}</Td>
                        <Td className="text-right">{s.total_market_value.toLocaleString()}</Td>
                        <Td className="text-right">{s.exchange_rate}</Td>
                        <Td className="text-right">{formatCAD(snapshotCadValue(s))}</Td>
                        <Td className="text-right whitespace-nowrap">
                          <button
                            type="button"
                            onClick={() => startEdit(s)}
                            className="mr-2 text-blue-500 hover:underline dark:text-blue-400"
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            onClick={() => deleteSnapshot(s.id)}
                            className="text-red-500 hover:underline dark:text-red-400"
                          >
                            Delete
                          </button>
                        </Td>
                      </>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function inputCls(): string {
  return "w-full rounded border border-gray-300 bg-white px-2 py-1 text-xs focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";
}

function SInput({
  label,
  value,
  onChange,
  type = "text",
  inputMode,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  inputMode?: string;
}): React.JSX.Element {
  return (
    <label className="flex flex-col text-xs">
      <span className="mb-1 text-gray-600 dark:text-gray-400">{label}</span>
      <input
        type={type}
        inputMode={
          inputMode as "decimal" | "numeric" | "tel" | "url" | "email" | "search" | undefined
        }
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={inputCls()}
      />
    </label>
  );
}

function ModeButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded px-2 py-0.5 transition-colors ${
        active
          ? "bg-blue-500 text-white dark:bg-blue-400 dark:text-gray-900"
          : "border border-gray-300 text-gray-600 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-800"
      }`}
    >
      {children}
    </button>
  );
}

function Th({ children, className }: { children?: React.ReactNode; className?: string }) {
  return (
    <th className={`px-2 py-1 font-medium text-gray-600 dark:text-gray-300 ${className ?? ""}`}>
      {children}
    </th>
  );
}

function Td({ children, className }: { children?: React.ReactNode; className?: string }) {
  return <td className={`px-2 py-1 align-middle ${className ?? ""}`}>{children}</td>;
}
