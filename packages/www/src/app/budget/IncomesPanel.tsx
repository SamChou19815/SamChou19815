"use client";

import { useEffect, useState } from "react";
import { getSupabase } from "../../lib/supabase";
import { Card } from "./BudgetApp";
import BulkAddPaste from "./BulkAddPaste";
import BulkBar, { BulkEditField, HeaderCheckbox, RowCheckbox } from "./BulkBar";
import type { Income } from "./types";
import { useBulkSelection } from "./useBulkSelection";
import {
  formatCAD,
  friendlyMutationError,
  parseNonNegFloat,
  splitPastedRows,
  todayISO,
} from "./utils";

type Props = {
  userId: string | null;
  rows: ReadonlyArray<Income>;
  loading: boolean;
  error: string | null;
};

type Draft = {
  name: string;
  source: string;
  amount: string;
  date: string;
  notes: string;
};

const emptyDraft = (): Draft => ({
  name: "",
  source: "",
  amount: "",
  date: todayISO(),
  notes: "",
});

type BulkEdit = {
  setName: boolean;
  name: string;
  setSource: boolean;
  source: string;
  setAmount: boolean;
  amount: string;
  setDate: boolean;
  date: string;
  setNotes: boolean;
  notes: string;
};

const emptyBulkEdit = (): BulkEdit => ({
  setName: false,
  name: "",
  setSource: false,
  source: "",
  setAmount: false,
  amount: "",
  setDate: false,
  date: todayISO(),
  setNotes: false,
  notes: "",
});

export default function IncomesPanel({ userId, rows, loading, error }: Props): React.JSX.Element {
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState<Draft>(emptyDraft);
  const [addMode, setAddMode] = useState<"single" | "bulk">("single");
  const [bulkEdit, setBulkEdit] = useState<BulkEdit>(emptyBulkEdit);

  const sel = useBulkSelection(rows);

  useEffect(() => {
    if (submitError == null) return;
    const t = setTimeout(() => setSubmitError(null), 5000);
    return () => clearTimeout(t);
  }, [submitError]);

  const addRow = async () => {
    if (userId == null) return;
    const name = draft.name.trim();
    const amount = parseNonNegFloat(draft.amount);
    if (name === "" || amount == null || draft.date === "") {
      setSubmitError("Name, amount (≥ 0), and date are required.");
      return;
    }
    setSubmitting(true);
    setSubmitError(null);
    const { error: err } = await getSupabase()
      .from("incomes")
      .insert({
        user_id: userId,
        name,
        source: draft.source.trim() === "" ? null : draft.source.trim(),
        amount,
        date: draft.date,
        notes: draft.notes.trim() === "" ? null : draft.notes.trim(),
      });
    setSubmitting(false);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }
    setDraft(emptyDraft());
  };

  const startEdit = (row: Income) => {
    setEditingId(row.id);
    setEditDraft({
      name: row.name,
      source: row.source ?? "",
      amount: String(row.amount),
      date: row.date,
      notes: row.notes ?? "",
    });
  };

  const saveEdit = async () => {
    if (editingId == null) return;
    const name = editDraft.name.trim();
    const amount = parseNonNegFloat(editDraft.amount);
    if (name === "" || amount == null || editDraft.date === "") {
      setSubmitError("Name, amount (≥ 0), and date are required.");
      return;
    }
    const { error: err } = await getSupabase()
      .from("incomes")
      .update({
        name,
        source: editDraft.source.trim() === "" ? null : editDraft.source.trim(),
        amount,
        date: editDraft.date,
        notes: editDraft.notes.trim() === "" ? null : editDraft.notes.trim(),
      })
      .eq("id", editingId);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }
    setEditingId(null);
  };

  const deleteRow = async (id: string) => {
    if (!window.confirm("Delete this income entry?")) return;
    const { error: err } = await getSupabase().from("incomes").delete().eq("id", id);
    if (err != null) setSubmitError(friendlyMutationError(err));
  };

  const bulkDelete = async () => {
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;
    const { error: err } = await getSupabase().from("incomes").delete().in("id", ids);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }
    sel.clear();
  };

  const bulkApply = async () => {
    const ids = sel.selectedRows.map((r) => r.id);
    if (ids.length === 0) return;
    const patch: Record<string, string | number | null> = {};
    if (bulkEdit.setName) {
      const v = bulkEdit.name.trim();
      if (v === "") {
        setSubmitError("Name cannot be blank.");
        return;
      }
      patch.name = v;
    }
    if (bulkEdit.setSource) {
      const v = bulkEdit.source.trim();
      patch.source = v === "" ? null : v;
    }
    if (bulkEdit.setAmount) {
      const a = parseNonNegFloat(bulkEdit.amount);
      if (a == null) {
        setSubmitError("Amount must be ≥ 0.");
        return;
      }
      patch.amount = a;
    }
    if (bulkEdit.setDate) {
      if (bulkEdit.date === "") {
        setSubmitError("Date is required.");
        return;
      }
      patch.date = bulkEdit.date;
    }
    if (bulkEdit.setNotes) {
      const v = bulkEdit.notes.trim();
      patch.notes = v === "" ? null : v;
    }
    if (Object.keys(patch).length === 0) {
      setSubmitError("Tick at least one field to apply.");
      return;
    }
    const { error: err } = await getSupabase().from("incomes").update(patch).in("id", ids);
    if (err != null) {
      setSubmitError(friendlyMutationError(err));
      return;
    }
    setBulkEdit(emptyBulkEdit());
  };

  const bulkInsert = async (text: string) => {
    if (userId == null) return { ok: false as const, message: "Not signed in." };
    const lines = splitPastedRows(text);
    type Payload = {
      user_id: string;
      name: string;
      source: string | null;
      amount: number;
      date: string;
      notes: string | null;
    };
    const payloads: Payload[] = [];
    for (let i = 0; i < lines.length; i++) {
      const cells = lines[i] ?? [];
      const [date = "", name = "", source = "", amountStr = "", notes = ""] = cells;
      const amount = parseNonNegFloat(amountStr);
      if (date === "" || name === "" || amount == null) {
        return {
          ok: false as const,
          message: `Line ${i + 1}: need date, name, and amount (≥ 0).`,
        };
      }
      payloads.push({
        user_id: userId,
        name,
        source: source === "" ? null : source,
        amount,
        date,
        notes: notes === "" ? null : notes,
      });
    }
    const { error: err } = await getSupabase().from("incomes").insert(payloads);
    if (err != null) return { ok: false as const, message: friendlyMutationError(err) };
    return { ok: true as const, insertedCount: payloads.length };
  };

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <div className="mb-4 flex items-center justify-between">
          <h3 className="m-0">Add income</h3>
          <div className="flex gap-1 text-xs">
            <ModeButton active={addMode === "single"} onClick={() => setAddMode("single")}>
              Single
            </ModeButton>
            <ModeButton active={addMode === "bulk"} onClick={() => setAddMode("bulk")}>
              Bulk paste
            </ModeButton>
          </div>
        </div>
        {(submitError || error) && (
          <div className="mb-3 rounded border border-red-400 bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
            {submitError ?? error}
          </div>
        )}
        {addMode === "single" ? (
          <>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-6">
              <Input
                label="Name"
                value={draft.name}
                onChange={(v) => setDraft({ ...draft, name: v })}
              />
              <Input
                label="Source"
                value={draft.source}
                onChange={(v) => setDraft({ ...draft, source: v })}
              />
              <Input
                label="Amount (CAD)"
                type="text"
                inputMode="decimal"
                value={draft.amount}
                onChange={(v) => setDraft({ ...draft, amount: v })}
              />
              <Input
                label="Date"
                type="date"
                value={draft.date}
                onChange={(v) => setDraft({ ...draft, date: v })}
              />
              <Input
                label="Notes"
                value={draft.notes}
                onChange={(v) => setDraft({ ...draft, notes: v })}
                className="sm:col-span-2"
              />
            </div>
            <div className="mt-4 flex justify-end">
              <button
                type="button"
                disabled={submitting}
                onClick={addRow}
                className="rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
              >
                {submitting ? "Adding…" : "Add"}
              </button>
            </div>
          </>
        ) : (
          <BulkAddPaste
            placeholder={
              "2026-04-01\tSalary\tEmployer\t5000\tApril paycheque\n2026-04-15\tSide gig\tFreelance\t300"
            }
            helpText={
              <span>
                One row per line: <code>date</code>, <code>name</code>,{" "}
                <code>source (optional)</code>, <code>amount</code>, <code>notes (optional)</code>.
                Tab- or comma-separated. Date must be <code>YYYY-MM-DD</code>.
              </span>
            }
            onSubmit={bulkInsert}
          />
        )}
      </Card>

      <Card>
        <h3 className="mb-4">Income</h3>
        <BulkBar
          count={sel.count}
          itemNoun="income entry"
          onDelete={bulkDelete}
          onClear={sel.clear}
          onApplyEdit={bulkApply}
          editForm={
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-5">
              <BulkEditField
                label="Name"
                set={bulkEdit.setName}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setName: v })}
              >
                <input
                  value={bulkEdit.name}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, name: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
              <BulkEditField
                label="Source"
                set={bulkEdit.setSource}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setSource: v })}
              >
                <input
                  value={bulkEdit.source}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, source: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
              <BulkEditField
                label="Amount (CAD)"
                set={bulkEdit.setAmount}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setAmount: v })}
              >
                <input
                  type="text"
                  inputMode="decimal"
                  value={bulkEdit.amount}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, amount: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
              <BulkEditField
                label="Date"
                set={bulkEdit.setDate}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setDate: v })}
              >
                <input
                  type="date"
                  value={bulkEdit.date}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, date: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
              <BulkEditField
                label="Notes"
                set={bulkEdit.setNotes}
                onSet={(v) => setBulkEdit({ ...bulkEdit, setNotes: v })}
              >
                <input
                  value={bulkEdit.notes}
                  onChange={(e) => setBulkEdit({ ...bulkEdit, notes: e.target.value })}
                  className={inputCls()}
                />
              </BulkEditField>
            </div>
          }
        />
        {loading && <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>}
        {!loading && rows.length === 0 && (
          <div className="text-center text-gray-500 dark:text-gray-400">
            No entries yet — add one above.
          </div>
        )}
        {!loading && rows.length > 0 && (
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
                  <Th>Date</Th>
                  <Th>Name</Th>
                  <Th>Source</Th>
                  <Th className="text-right">Amount</Th>
                  <Th>Notes</Th>
                  <Th />
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => {
                  const editing = editingId === row.id;
                  return (
                    <tr key={row.id} className="border-b border-gray-100 dark:border-gray-800">
                      <Td className="w-6">
                        <RowCheckbox
                          checked={sel.isSelected(row.id)}
                          onToggle={() => sel.toggle(row.id)}
                        />
                      </Td>
                      {editing ? (
                        <>
                          <Td>
                            <input
                              type="date"
                              value={editDraft.date}
                              onChange={(e) => setEditDraft({ ...editDraft, date: e.target.value })}
                              className={inputCls()}
                            />
                          </Td>
                          <Td>
                            <input
                              value={editDraft.name}
                              onChange={(e) => setEditDraft({ ...editDraft, name: e.target.value })}
                              className={inputCls()}
                            />
                          </Td>
                          <Td>
                            <input
                              value={editDraft.source}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, source: e.target.value })
                              }
                              className={inputCls()}
                            />
                          </Td>
                          <Td className="text-right">
                            <input
                              type="text"
                              inputMode="decimal"
                              value={editDraft.amount}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, amount: e.target.value })
                              }
                              className={`${inputCls()} text-right`}
                            />
                          </Td>
                          <Td>
                            <input
                              value={editDraft.notes}
                              onChange={(e) =>
                                setEditDraft({ ...editDraft, notes: e.target.value })
                              }
                              className={inputCls()}
                            />
                          </Td>
                          <Td className="text-right">
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
                          <Td>{row.date}</Td>
                          <Td>{row.name}</Td>
                          <Td>{row.source ?? ""}</Td>
                          <Td className="text-right">{formatCAD(row.amount)}</Td>
                          <Td className="text-gray-500 dark:text-gray-400">{row.notes ?? ""}</Td>
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

function inputCls(): string {
  return "w-full rounded border border-gray-300 bg-white px-2 py-1 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";
}

function Input({
  label,
  value,
  onChange,
  type = "text",
  inputMode,
  className,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  inputMode?: string;
  className?: string;
}): React.JSX.Element {
  return (
    <label className={`flex flex-col text-sm ${className ?? ""}`}>
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

function Td({ children, className }: { children?: React.ReactNode; className?: string }) {
  return <td className={`px-2 py-2 align-middle ${className ?? ""}`}>{children}</td>;
}
