import { useState } from "react";
import { getSupabase } from "../../lib/supabase";
import type { Investment } from "./types";
import {
  formatCAD,
  friendlyMutationError,
  localInputValueToISO,
  nowLocalInputValue,
  parseNonNegFloat,
  parsePosFloat,
} from "./utils";

type RowDraft = {
  total_cost: string;
  total_market_value: string;
  exchange_rate: string;
};

type Parsed = {
  id: string;
  total_cost: number;
  total_market_value: number;
  exchange_rate: number;
};

/**
 * Re-snapshots every investment in one go: each row starts at the investment's current
 * values, the user updates whatever moved, and saving updates every investment and records
 * a snapshot for each at the same timestamp.
 */
export default function MassSnapshotForm({
  userId,
  investments,
  onClose,
}: {
  userId: string;
  investments: ReadonlyArray<Investment>;
  onClose: () => void;
}): React.JSX.Element {
  // Pre-populated once when the form opens, so live updates never clobber in-progress edits.
  const [drafts, setDrafts] = useState<Record<string, RowDraft>>(() =>
    Object.fromEntries(
      investments.map((inv) => [
        inv.id,
        {
          total_cost: String(inv.total_cost),
          total_market_value: String(inv.total_market_value),
          exchange_rate: String(inv.exchange_rate),
        },
      ]),
    ),
  );
  const [recordedAt, setRecordedAt] = useState(nowLocalInputValue);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const setField = (id: string, field: keyof RowDraft, value: string) => {
    setDrafts((prev) => {
      const row = prev[id];
      return row == null ? prev : { ...prev, [id]: { ...row, [field]: value } };
    });
  };

  const parseRow = (inv: Investment): Parsed | string => {
    const d = drafts[inv.id];
    if (d == null) return `${inv.name}: missing values.`;
    const cost = parseNonNegFloat(d.total_cost);
    if (cost == null) return `${inv.name}: total cost must be ≥ 0.`;
    const mv = parseNonNegFloat(d.total_market_value);
    if (mv == null) return `${inv.name}: market value must be ≥ 0.`;
    const fx = inv.currency === "CAD" ? 1 : parsePosFloat(d.exchange_rate);
    if (fx == null) return `${inv.name}: exchange rate must be > 0.`;
    return { id: inv.id, total_cost: cost, total_market_value: mv, exchange_rate: fx };
  };

  const save = async () => {
    if (recordedAt === "") {
      setError("Recorded at is required.");
      return;
    }
    const parsed: Parsed[] = [];
    for (const inv of investments) {
      const p = parseRow(inv);
      if (typeof p === "string") {
        setError(p);
        return;
      }
      parsed.push(p);
    }
    setSaving(true);
    setError(null);
    const updated_at = localInputValueToISO(recordedAt);

    // One insert, so either every snapshot is recorded or none are.
    const { error: snapErr } = await getSupabase()
      .from("investment_snapshots")
      .insert(
        parsed.map((p) => ({
          user_id: userId,
          investment_id: p.id,
          total_cost: p.total_cost,
          total_market_value: p.total_market_value,
          exchange_rate: p.exchange_rate,
          recorded_at: updated_at,
        })),
      );
    if (snapErr != null) {
      setSaving(false);
      setError(friendlyMutationError(snapErr));
      return;
    }

    const results = await Promise.all(
      parsed.map((p) =>
        getSupabase()
          .from("investments")
          .update({
            total_cost: p.total_cost,
            total_market_value: p.total_market_value,
            exchange_rate: p.exchange_rate,
            updated_at,
          })
          .eq("id", p.id),
      ),
    );
    setSaving(false);
    const failed = results.flatMap((r, i) =>
      r.error == null ? [] : [investments[i]?.name ?? parsed[i]?.id],
    );
    const firstErr = results.find((r) => r.error != null)?.error;
    if (firstErr != null) {
      setError(
        `Snapshots saved, but updating ${failed.join(", ")} failed: ${friendlyMutationError(firstErr)}`,
      );
      return;
    }
    onClose();
  };

  return (
    <div className="mb-4 rounded border border-blue-300 bg-blue-50 px-3 py-3 dark:border-blue-700 dark:bg-blue-900/20">
      <div className="mb-3 flex flex-wrap items-end justify-between gap-3">
        <p className="m-0 text-sm text-gray-600 dark:text-gray-400">
          Update whatever changed, then save to snapshot all {investments.length} investments at
          once.
        </p>
        <label className="flex flex-col text-sm">
          <span className="mb-1 text-gray-600 dark:text-gray-400">Recorded at</span>
          <input
            type="datetime-local"
            value={recordedAt}
            onChange={(e) => setRecordedAt(e.target.value)}
            className={inputCls()}
          />
        </label>
      </div>
      {error && (
        <div className="mb-3 rounded border border-red-400 bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
          {error}
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="min-w-full text-sm">
          <thead>
            <tr className="border-b border-blue-200 text-left dark:border-blue-700">
              <Th>Name</Th>
              <Th>Currency</Th>
              <Th className="text-right">Total cost</Th>
              <Th className="text-right">Market value</Th>
              <Th className="text-right">FX→CAD</Th>
              <Th className="text-right">Value (CAD)</Th>
            </tr>
          </thead>
          <tbody>
            {investments.map((inv) => {
              const d = drafts[inv.id];
              if (d == null) return null;
              const p = parseRow(inv);
              const before = inv.total_market_value * inv.exchange_rate;
              const after = typeof p === "string" ? null : p.total_market_value * p.exchange_rate;
              const delta = after == null ? 0 : after - before;
              return (
                <tr key={inv.id} className="border-b border-blue-100 dark:border-blue-900/50">
                  <Td>{inv.name}</Td>
                  <Td>{inv.currency}</Td>
                  <Td>
                    <NumInput
                      value={d.total_cost}
                      changed={d.total_cost !== String(inv.total_cost)}
                      onChange={(v) => setField(inv.id, "total_cost", v)}
                    />
                  </Td>
                  <Td>
                    <NumInput
                      value={d.total_market_value}
                      changed={d.total_market_value !== String(inv.total_market_value)}
                      onChange={(v) => setField(inv.id, "total_market_value", v)}
                    />
                  </Td>
                  <Td>
                    <NumInput
                      value={inv.currency === "CAD" ? "1" : d.exchange_rate}
                      disabled={inv.currency === "CAD"}
                      changed={d.exchange_rate !== String(inv.exchange_rate)}
                      onChange={(v) => setField(inv.id, "exchange_rate", v)}
                    />
                  </Td>
                  <Td className="text-right whitespace-nowrap">
                    {after == null ? "—" : formatCAD(after)}
                    {after != null && delta !== 0 && (
                      <span
                        className={`ml-1 text-xs ${
                          delta > 0
                            ? "text-green-600 dark:text-green-400"
                            : "text-red-600 dark:text-red-400"
                        }`}
                      >
                        ({delta > 0 ? "+" : "-"}
                        {formatCAD(Math.abs(delta))})
                      </span>
                    )}
                  </Td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="mt-3 flex justify-end gap-2">
        <button
          type="button"
          onClick={onClose}
          disabled={saving}
          className="rounded border border-gray-300 px-3 py-1 text-sm font-medium hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={save}
          disabled={saving}
          className="rounded bg-blue-500 px-4 py-1 text-sm font-bold text-white hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
        >
          {saving ? "Saving…" : `Snapshot ${investments.length}`}
        </button>
      </div>
    </div>
  );
}

function NumInput({
  value,
  onChange,
  changed,
  disabled,
}: {
  value: string;
  onChange: (v: string) => void;
  changed: boolean;
  disabled?: boolean;
}): React.JSX.Element {
  return (
    <input
      type="text"
      inputMode="decimal"
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
      className={`${inputCls()} min-w-24 text-right disabled:opacity-60 ${
        changed && !disabled ? "border-blue-500 dark:border-blue-400" : ""
      }`}
    />
  );
}

function inputCls(): string {
  return "w-full rounded border border-gray-300 bg-white px-2 py-1 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";
}

function Th({ children, className }: { children?: React.ReactNode; className?: string }) {
  return (
    <th className={`px-2 py-2 font-medium text-gray-600 dark:text-gray-300 ${className ?? ""}`}>
      {children}
    </th>
  );
}

function Td({ children, className }: { children?: React.ReactNode; className?: string }) {
  return <td className={`px-2 py-1 align-middle ${className ?? ""}`}>{children}</td>;
}
