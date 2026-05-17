"use client";

import { useState, type ReactNode } from "react";

export type ParseResult<R> =
  | { ok: true; rows: ReadonlyArray<R> }
  | { ok: false; line: number; message: string };

export type PreviewColumn<R> = {
  header: string;
  cell: (row: R) => ReactNode;
  align?: "right";
};

type Result = { ok: true; insertedCount: number } | { ok: false; message: string };

type Props<R> = {
  helpText: ReactNode;
  placeholder: string;
  columns: ReadonlyArray<PreviewColumn<R>>;
  parse: (text: string) => ParseResult<R>;
  onInsert: (rows: ReadonlyArray<R>) => Promise<Result>;
};

export default function BulkAddPaste<R>({
  helpText,
  placeholder,
  columns,
  parse,
  onInsert,
}: Props<R>): React.JSX.Element {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ kind: "ok" | "err"; message: string } | null>(null);

  const preview = text.trim() === "" ? null : parse(text);

  const submit = async () => {
    if (preview == null) {
      setStatus({ kind: "err", message: "Paste at least one row first." });
      return;
    }
    if (!preview.ok) {
      setStatus({ kind: "err", message: `Line ${preview.line}: ${preview.message}` });
      return;
    }
    setBusy(true);
    setStatus(null);
    const result = await onInsert(preview.rows);
    setBusy(false);
    if (result.ok) {
      setStatus({
        kind: "ok",
        message: `Inserted ${result.insertedCount} ${result.insertedCount === 1 ? "row" : "rows"}.`,
      });
      setText("");
    } else {
      setStatus({ kind: "err", message: result.message });
    }
  };

  const insertLabel =
    preview != null && preview.ok
      ? `Insert ${preview.rows.length} ${preview.rows.length === 1 ? "row" : "rows"}`
      : "Insert all";

  return (
    <div className="flex flex-col gap-2">
      <div className="text-xs text-gray-600 dark:text-gray-400">{helpText}</div>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={placeholder}
        rows={6}
        className="w-full rounded border border-gray-300 bg-white p-2 font-mono text-xs focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
      />
      {preview != null && <Preview preview={preview} columns={columns} />}
      {status && (
        <div
          className={`rounded px-3 py-1 text-sm ${
            status.kind === "ok"
              ? "bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-300"
              : "border border-red-400 bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"
          }`}
        >
          {status.message}
        </div>
      )}
      <div className="flex justify-end">
        <button
          type="button"
          onClick={submit}
          disabled={busy}
          className="rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
        >
          {busy ? "Inserting…" : insertLabel}
        </button>
      </div>
    </div>
  );
}

function Preview<R>({
  preview,
  columns,
}: {
  preview: ParseResult<R>;
  columns: ReadonlyArray<PreviewColumn<R>>;
}): React.JSX.Element {
  if (!preview.ok) {
    return (
      <div className="rounded border border-amber-400 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-500/40 dark:bg-amber-900/20 dark:text-amber-200">
        Preview error — line {preview.line}: {preview.message}
      </div>
    );
  }
  return (
    <div className="overflow-hidden rounded border border-gray-200 dark:border-gray-700">
      <div className="border-b border-gray-200 bg-gray-50 px-3 py-1 text-xs text-gray-600 dark:border-gray-700 dark:bg-gray-800/60 dark:text-gray-300">
        Preview — {preview.rows.length} {preview.rows.length === 1 ? "row" : "rows"}
      </div>
      <div className="max-h-64 overflow-auto">
        <table className="min-w-full text-xs">
          <thead className="sticky top-0 bg-white dark:bg-gray-900">
            <tr className="border-b border-gray-200 text-left dark:border-gray-700">
              {columns.map((c) => (
                <th
                  key={c.header}
                  className={`px-2 py-1 font-medium text-gray-600 dark:text-gray-300 ${
                    c.align === "right" ? "text-right" : ""
                  }`}
                >
                  {c.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((row, i) => (
              // oxlint-disable-next-line no-array-index-key
              <tr key={i} className="border-b border-gray-100 last:border-0 dark:border-gray-800">
                {columns.map((c) => (
                  <td
                    key={c.header}
                    className={`px-2 py-1 align-middle ${c.align === "right" ? "text-right" : ""}`}
                  >
                    {c.cell(row)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
