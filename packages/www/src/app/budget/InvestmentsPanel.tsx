"use client";

import { useEffect, useState } from "react";
import { getSupabase } from "../../lib/supabase";
import { Card } from "./BudgetApp";
import BulkAddPaste from "./BulkAddPaste";
import BulkBar, { BulkEditField, HeaderCheckbox, RowCheckbox } from "./BulkBar";
import SnapshotsHistory from "./SnapshotsHistory";
import type { Currency, Investment, InvestmentSnapshot } from "./types";
import { useBulkSelection } from "./useBulkSelection";
import {
  cadValue,
  formatCAD,
  friendlyMutationError,
  isoToLocalInputValue,
  localInputValueToISO,
  nowLocalInputValue,
  parseNonNegFloat,
  parsePosFloat,
  splitPastedRows,
} from "./utils";

type Props = {
  userId: string | null;
  investments: ReadonlyArray<Investment>;
  investmentsLoading: boolean;
  investmentsError: string | null;
  snapshots: ReadonlyArray<InvestmentSnapshot>;
  snapshotsLoading: boolean;
};

type Draft = {
  name: string;
  type: string;
  currency: Currency;
  total_cost: string;
  total_market_value: string;
  exchange_rate: string;
  updated_at: string;
};

const emptyDraft = (): Draft => ({
  name: "",
  type: "",
  currency: "CAD",
  total_cost: "",
  total_market_value: "",
  exchange_rate: "1",
  updated_at: nowLocalInputValue(),
});

type BulkEdit = {
  setType: boolean;
  type: string;
  setCurrency: boolean;
  currency: Currency;
  setTotalCost: boolean;
  total_cost: string;
  setTotalMarketValue: boolean;
  total_market_value: string;
  setExchangeRate: boolean;
  exchange_rate: string;
  setUpdatedAt: boolean;
  updated_at: string;
};

const emptyBulkEdit = (): BulkEdit => ({
  setType: false,
  type: "",
  setCurrency: false,
  currency: "CAD",
  setTotalCost: false,
  total_cost: "",
  setTotalMarketValue: false,
  total_market_value: "",
  setExchangeRate: false,
  exchange_rate: "1",
  setUpdatedAt: false,
  updated_at: nowLocalInputValue(),
});

export default function InvestmentsPanel({
  userId,
  investments,
  investmentsLoading,
  investmentsError,
  snapshots,
  snapshotsLoading,
}: Props): React.JSX.Element {
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<Draft>(emptyDraft);
  const [addMode, setAddMode] = useState<"single" | "bulk">("single");
  const [bulkEdit, setBulkEdit] = useState<BulkEdit>(emptyBulkEdit);

  const sel = useBulkSelection(investments);

  useEffect(() => {
    if (submitError == null) return;
    const t = setTimeout(() => setSubmitError(null), 5000);
    return () => clearTimeout(t);
  }, [submitError]);

  const validateDraft = (d: Draft): string | null => {
    if (d.name.trim() === "") return "Name is required.";
    if (d.type.trim() === "") return "Type is required.";
    const cost = parseNonNegFloat(d.total_cost);
    const value = parseNonNegFloat(d.total_market_value);
    if (cost == null) return "Total cost must be ≥ 0.";
    if (value == null) return "Total market value must be ≥ 0.";
    if (d.currency === "CAD") {
      if (Number(d.exchange_rate.replace(/,/g, "")) !== 1)
        return "Exchange rate must be 1 for CAD.";
    } else {
      const rate = parsePosFloat(d.exchange_rate);
      if (rate == null) return "Exchange rate must be > 0.";
    }
    if (d.updated_at === "") return "Updated at is required.";
    return null;
  };

  const writeSnapshot = async (
    investment: Pick<
      Investment,
      "id" | "user_id" | "total_cost" | "total_market_value" | "exchange_rate" | "updated_at"
    >,
  ) => {
    await getSupabase().from("investment_snapshots").insert({
      user_id: investment.user_id,
      investment_id: investment.id,
      total_cost: investment.total_cost,
      total_market_value: investment.total_market_value,
      exchange_rate: investment.exchange_rate,
      recorded_at: investment.updated_at,
    });
  };

  const addInvestment = async () => {
    if (userId == null) return;
    const err = validateDraft(draft);
    if (err != null) {
      setSubmitError(err);
      return;
    }
    setSubmitting(true);
    setSubmitError(null);
    const updated_at = localInputValueToISO(draft.updated_at);
    const { data, error: insertErr } = await getSupabase()
      .from("investments")
      .insert({
        user_id: userId,
        name: draft.name.trim(),
        type: draft.type.trim(),
        currency: draft.currency,
        total_cost: Number(draft.total_cost.replace(/,/g, "")),
        total_market_value: Number(draft.total_market_value.replace(/,/g, "")),
        exchange_rate: draft.currency === "CAD" ? 1 : Number(draft.exchange_rate.replace(/,/g, "")),
        updated_at,
      })
      .select()
      .single();
    if (insertErr != null || data == null) {
      setSubmitting(false);
      setSubmitError(insertErr != null ? friendlyMutationError(insertErr) : "Insert failed.");
      return;
    }
    await writeSnapshot(data as Investment);
    setSubmitting(false);
    setDraft(emptyDraft());
  };

  const startEdit = (row: Investment) => {
    setEditingId(row.id);
    setEditDraft({
      name: row.name,
      type: row.type,
      currency: row.currency,
      total_cost: String(row.total_cost),
      total_market_value: String(row.total_market_value),
      exchange_rate: String(row.exchange_rate),
      updated_at: isoToLocalInputValue(row.updated_at),
    });
  };

  const saveEdit = async () => {
    if (editingId == null || userId == null) return;
    const err = validateDraft(editDraft);
    if (err != null) {
      setSubmitError(err);
      return;
    }
    const updated_at = localInputValueToISO(editDraft.updated_at);
    const payload = {
      name: editDraft.name.trim(),
      type: editDraft.type.trim(),
      currency: editDraft.currency,
      total_cost: Number(editDraft.total_cost.replace(/,/g, "")),
      total_market_value: Number(editDraft.total_market_value.replace(/,/g, "")),
      exchange_rate:
        editDraft.currency === "CAD" ? 1 : Number(editDraft.exchange_rate.replace(/,/g, "")),
      updated_at,
    };
    const { data, error: updErr } = await getSupabase()
      .from("investments")
      .update(payload)
      .eq("id", editingId)
      .select()
      .single();
    if (updErr != null || data == null) {
      setSubmitError(updErr != null ? friendlyMutationError(updErr) : "Update failed.");
      return;
    }
    await writeSnapshot(data as Investment);
    setEditingId(null);
  };

  const deleteRow = async (id: string) => {
    if (!window.confirm("Delete this investment and all its snapshots?")) return;
    const { error: err } = await getSupabase().from("investments").delete().eq("id", id);
    if (err != null) setSubmitError(friendlyMutationError(err));
  };

  const toggleExpand = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const bulkDelete = async () => {
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;
    const { error: err } = await getSupabase().from("investments").delete().in("id", ids);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }
    sel.clear();
  };

  const bulkApply = async () => {
    if (userId == null) return;
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;

    const patch: {
      type?: string;
      currency?: Currency;
      total_cost?: number;
      total_market_value?: number;
      exchange_rate?: number;
      updated_at?: string;
    } = {};

    if (bulkEdit.setType) {
      const v = bulkEdit.type.trim();
      if (v === "") {
        setSubmitError("Type cannot be blank.");
        return;
      }
      patch.type = v;
    }
    if (bulkEdit.setCurrency) {
      patch.currency = bulkEdit.currency;
    }
    if (bulkEdit.setTotalCost) {
      const c = parseNonNegFloat(bulkEdit.total_cost);
      if (c == null) {
        setSubmitError("Total cost must be ≥ 0.");
        return;
      }
      patch.total_cost = c;
    }
    if (bulkEdit.setTotalMarketValue) {
      const v = parseNonNegFloat(bulkEdit.total_market_value);
      if (v == null) {
        setSubmitError("Total market value must be ≥ 0.");
        return;
      }
      patch.total_market_value = v;
    }
    if (bulkEdit.setExchangeRate) {
      const r = parsePosFloat(bulkEdit.exchange_rate);
      if (r == null) {
        setSubmitError("Exchange rate must be > 0.");
        return;
      }
      patch.exchange_rate = r;
    }
    if (bulkEdit.setUpdatedAt) {
      if (bulkEdit.updated_at === "") {
        setSubmitError("Updated at is required.");
        return;
      }
      patch.updated_at = localInputValueToISO(bulkEdit.updated_at);
    }
    // CAD must always have rate=1.
    if (patch.currency === "CAD") patch.exchange_rate = 1;

    if (Object.keys(patch).length === 0) {
      setSubmitError("Tick at least one field to apply.");
      return;
    }

    const { error: err } = await getSupabase().from("investments").update(patch).in("id", ids);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }

    // Snapshot whenever any financial field (or the timestamp) changed.
    const financialChanged =
      bulkEdit.setTotalCost ||
      bulkEdit.setTotalMarketValue ||
      bulkEdit.setExchangeRate ||
      bulkEdit.setCurrency ||
      bulkEdit.setUpdatedAt;
    if (financialChanged) {
      const snapshotRows = sel.selectedRows.map((row) => {
        const merged = { ...row, ...patch };
        return {
          user_id: userId,
          investment_id: row.id,
          total_cost: merged.total_cost,
          total_market_value: merged.total_market_value,
          exchange_rate: merged.exchange_rate,
          recorded_at: merged.updated_at,
        };
      });
      const { error: snapErr } = await getSupabase()
        .from("investment_snapshots")
        .insert(snapshotRows);
      if (snapErr != null) {
        setSubmitError(`Update saved but snapshots failed: ${friendlyMutationError(snapErr)}`);
        return;
      }
    }

    setBulkEdit(emptyBulkEdit());
  };

  const bulkInsert = async (text: string) => {
    if (userId == null) return { ok: false as const, message: "Not signed in." };
    const lines = splitPastedRows(text);
    type Payload = {
      user_id: string;
      name: string;
      type: string;
      currency: Currency;
      total_cost: number;
      total_market_value: number;
      exchange_rate: number;
      updated_at: string;
    };
    const payloads: Payload[] = [];
    for (let i = 0; i < lines.length; i++) {
      const cells = lines[i] ?? [];
      const [
        name = "",
        type = "",
        currencyStr = "",
        costStr = "",
        mvStr = "",
        fxStr = "",
        updatedStr = "",
      ] = cells;
      const currency = currencyStr.toUpperCase();
      if (currency !== "CAD" && currency !== "USD") {
        return { ok: false as const, message: `Line ${i + 1}: currency must be CAD or USD.` };
      }
      const cost = parseNonNegFloat(costStr);
      const mv = parseNonNegFloat(mvStr);
      const fx = currency === "CAD" ? 1 : parsePosFloat(fxStr);
      if (name === "" || type === "" || cost == null || mv == null || fx == null) {
        return {
          ok: false as const,
          message: `Line ${i + 1}: need name, type, currency, total_cost (≥ 0), total_market_value (≥ 0), exchange_rate (> 0).`,
        };
      }
      const parsed = new Date(updatedStr);
      if (updatedStr === "" || Number.isNaN(parsed.getTime())) {
        return { ok: false as const, message: `Line ${i + 1}: invalid updated_at.` };
      }
      payloads.push({
        user_id: userId,
        name,
        type,
        currency: currency as Currency,
        total_cost: cost,
        total_market_value: mv,
        exchange_rate: fx,
        updated_at: parsed.toISOString(),
      });
    }
    const { data, error: err } = await getSupabase().from("investments").insert(payloads).select();
    if (err != null || data == null) {
      return {
        ok: false as const,
        message: err != null ? friendlyMutationError(err) : "Insert failed.",
      };
    }
    const inserted = data as Investment[];
    const snapRows = inserted.map((inv) => ({
      user_id: inv.user_id,
      investment_id: inv.id,
      total_cost: inv.total_cost,
      total_market_value: inv.total_market_value,
      exchange_rate: inv.exchange_rate,
      recorded_at: inv.updated_at,
    }));
    await getSupabase().from("investment_snapshots").insert(snapRows);
    return { ok: true as const, insertedCount: inserted.length };
  };

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <div className="mb-4 flex items-center justify-between">
          <h3 className="m-0">Add investment</h3>
          <div className="flex gap-1 text-xs">
            <ModeButton active={addMode === "single"} onClick={() => setAddMode("single")}>
              Single
            </ModeButton>
            <ModeButton active={addMode === "bulk"} onClick={() => setAddMode("bulk")}>
              Bulk paste
            </ModeButton>
          </div>
        </div>
        {(submitError || investmentsError) && (
          <div className="mb-3 rounded border border-red-400 bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
            {submitError ?? investmentsError}
          </div>
        )}
        {addMode === "single" ? (
          <>
            <DraftForm draft={draft} onChange={setDraft} />
            <div className="mt-4 flex justify-end">
              <button
                type="button"
                disabled={submitting}
                onClick={addInvestment}
                className="rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
              >
                {submitting ? "Adding…" : "Add"}
              </button>
            </div>
          </>
        ) : (
          <BulkAddPaste
            placeholder={
              "TFSA Wealthsimple\tStocks\tCAD\t10000\t11250\t1\t2026-04-01T09:00\nUS Index\tETF\tUSD\t5000\t5400\t1.36\t2026-04-01T09:00"
            }
            helpText={
              <span>
                One row per line: <code>name</code>, <code>type</code>,{" "}
                <code>currency (CAD|USD)</code>, <code>total_cost</code>,{" "}
                <code>total_market_value</code>, <code>exchange_rate</code>, <code>updated_at</code>
                . Tab- or comma-separated. <code>updated_at</code> can be an ISO timestamp or{" "}
                <code>YYYY-MM-DDTHH:MM</code>. CAD rows always store rate=1. An initial snapshot is
                created for each row.
              </span>
            }
            onSubmit={bulkInsert}
          />
        )}
      </Card>

      <Card>
        <h3 className="mb-4">Investments</h3>
        <BulkBar
          count={sel.count}
          itemNoun="investment"
          onDelete={bulkDelete}
          onClear={sel.clear}
          onApplyEdit={bulkApply}
          editForm={
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3 lg:grid-cols-6">
              <BulkEditField
                label="Type"
                set={bulkEdit.setType}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setType: v })}
              >
                <input
                  value={bulkEdit.type}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, type: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
              <BulkEditField
                label="Currency"
                set={bulkEdit.setCurrency}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setCurrency: v })}
              >
                <select
                  value={bulkEdit.currency}
                  onChange={(e) =>
                    setBulkEdit({ ...bulkEdit, currency: e.target.value as Currency })
                  }
                  className={inputCls()}
                >
                  <option value="CAD">CAD</option>
                  <option value="USD">USD</option>
                </select>
              </BulkEditField>
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
                label="Total market value"
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
                label="Updated at"
                set={bulkEdit.setUpdatedAt}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setUpdatedAt: v })}
              >
                <input
                  type="datetime-local"
                  value={bulkEdit.updated_at}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, updated_at: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
            </div>
          }
        />
        {investmentsLoading && (
          <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
        )}
        {!investmentsLoading && investments.length === 0 && (
          <div className="text-center text-gray-500 dark:text-gray-400">
            No entries yet — add one above.
          </div>
        )}
        {!investmentsLoading && investments.length > 0 && (
          <div className="overflow-x-auto">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 text-left dark:border-gray-700">
                  <Th className="w-6">
                    <HeaderCheckbox
                      allSelected={sel.allSelected}
                      someSelected={sel.someSelected}
                      onToggle={sel.toggleAll}
                    />
                  </Th>
                  <Th />
                  <Th>Name</Th>
                  <Th>Type</Th>
                  <Th>Currency</Th>
                  <Th className="text-right">Cost</Th>
                  <Th className="text-right">Market value</Th>
                  <Th className="text-right">FX→CAD</Th>
                  <Th className="text-right">Value (CAD)</Th>
                  <Th>Updated</Th>
                  <Th />
                </tr>
              </thead>
              <tbody>
                {investments.map((row) => {
                  const editing = editingId === row.id;
                  const isOpen = expanded.has(row.id);
                  return (
                    <FragmentRow key={row.id}>
                      <tr className="border-b border-gray-100 dark:border-gray-800">
                        <Td className="w-6">
                          <RowCheckbox
                            checked={sel.isSelected(row.id)}
                            onToggle={() => sel.toggle(row.id)}
                          />
                        </Td>
                        <Td className="w-6">
                          <button
                            type="button"
                            aria-label={isOpen ? "Collapse" : "Expand"}
                            onClick={() => toggleExpand(row.id)}
                            className="text-gray-500 hover:text-blue-500 dark:text-gray-400 dark:hover:text-blue-400"
                          >
                            {isOpen ? "▾" : "▸"}
                          </button>
                        </Td>
                        {editing ? (
                          <Td className="" colSpan={9}>
                            <DraftForm draft={editDraft} onChange={setEditDraft} />
                            <div className="mt-2 flex justify-end gap-2">
                              <button
                                type="button"
                                onClick={saveEdit}
                                className="rounded bg-blue-500 px-3 py-1 text-sm font-bold text-white hover:bg-blue-600 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
                              >
                                Save
                              </button>
                              <button
                                type="button"
                                onClick={() => setEditingId(null)}
                                className="rounded border border-gray-300 px-3 py-1 text-sm font-medium hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
                              >
                                Cancel
                              </button>
                            </div>
                          </Td>
                        ) : (
                          <>
                            <Td>{row.name}</Td>
                            <Td>{row.type}</Td>
                            <Td>{row.currency}</Td>
                            <Td className="text-right">{row.total_cost.toLocaleString()}</Td>
                            <Td className="text-right">
                              {row.total_market_value.toLocaleString()}
                            </Td>
                            <Td className="text-right">{row.exchange_rate}</Td>
                            <Td className="text-right">{formatCAD(cadValue(row))}</Td>
                            <Td>{new Date(row.updated_at).toLocaleString()}</Td>
                            <Td className="text-right whitespace-nowrap">
                              <button
                                type="button"
                                onClick={() => startEdit(row)}
                                className="mr-2 text-blue-500 hover:underline dark:text-blue-400"
                              >
                                Edit
                              </button>
                              <button
                                type="button"
                                onClick={() => deleteRow(row.id)}
                                className="text-red-500 hover:underline dark:text-red-400"
                              >
                                Delete
                              </button>
                            </Td>
                          </>
                        )}
                      </tr>
                      {isOpen && (
                        <tr className="bg-gray-50 dark:bg-gray-900/30">
                          <Td colSpan={11}>
                            <SnapshotsHistory
                              userId={userId}
                              investmentId={row.id}
                              snapshots={snapshots.filter((s) => s.investment_id === row.id)}
                              loading={snapshotsLoading}
                            />
                          </Td>
                        </tr>
                      )}
                    </FragmentRow>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}

function FragmentRow({ children }: { children: React.ReactNode }): React.JSX.Element {
  return <>{children}</>;
}

function DraftForm({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (d: Draft) => void;
}): React.JSX.Element {
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-7">
      <Input label="Name" value={draft.name} onChange={(v) => onChange({ ...draft, name: v })} />
      <Input label="Type" value={draft.type} onChange={(v) => onChange({ ...draft, type: v })} />
      <label className="flex flex-col text-sm">
        <span className="mb-1 text-gray-600 dark:text-gray-400">Currency</span>
        <select
          value={draft.currency}
          onChange={(e) => {
            const cur = e.target.value as Currency;
            onChange({
              ...draft,
              currency: cur,
              exchange_rate: cur === "CAD" ? "1" : draft.exchange_rate,
            });
          }}
          className={inputCls()}
        >
          <option value="CAD">CAD</option>
          <option value="USD">USD</option>
        </select>
      </label>
      <Input
        label="Total cost"
        type="text"
        inputMode="decimal"
        value={draft.total_cost}
        onChange={(v) => onChange({ ...draft, total_cost: v })}
      />
      <Input
        label="Total market value"
        type="text"
        inputMode="decimal"
        value={draft.total_market_value}
        onChange={(v) => onChange({ ...draft, total_market_value: v })}
      />
      <label className="flex flex-col text-sm">
        <span className="mb-1 text-gray-600 dark:text-gray-400">FX → CAD</span>
        <input
          type="text"
          inputMode="decimal"
          step="0.0001"
          value={draft.currency === "CAD" ? "1" : draft.exchange_rate}
          disabled={draft.currency === "CAD"}
          onChange={(e) => onChange({ ...draft, exchange_rate: e.target.value })}
          className={`${inputCls()} disabled:opacity-60`}
        />
      </label>
      <Input
        label="Updated at"
        type="datetime-local"
        value={draft.updated_at}
        onChange={(v) => onChange({ ...draft, updated_at: v })}
      />
    </div>
  );
}

function inputCls(): string {
  return "w-full rounded border border-gray-300 bg-white px-2 py-1 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";
}

function Input({
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
    <label className="flex flex-col text-sm">
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
      className={`rounded px-2 py-1 transition-colors ${
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
    <th className={`px-2 py-2 font-medium text-gray-600 dark:text-gray-300 ${className ?? ""}`}>
      {children}
    </th>
  );
}

function Td({
  children,
  className,
  colSpan,
}: {
  children?: React.ReactNode;
  className?: string;
  colSpan?: number;
}) {
  return (
    <td className={`px-2 py-2 align-middle ${className ?? ""}`} colSpan={colSpan}>
      {children}
    </td>
  );
}
