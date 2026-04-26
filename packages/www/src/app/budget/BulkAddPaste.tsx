"use client";

import { useState, type ReactNode } from "react";

type Result = { ok: true; insertedCount: number } | { ok: false; message: string };

type Props = {
  helpText: ReactNode;
  placeholder: string;
  onSubmit: (text: string) => Promise<Result>;
};

export default function BulkAddPaste({
  helpText,
  placeholder,
  onSubmit,
}: Props): React.JSX.Element {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ kind: "ok" | "err"; message: string } | null>(null);

  const submit = async () => {
    if (text.trim() === "") {
      setStatus({ kind: "err", message: "Paste at least one row first." });
      return;
    }
    setBusy(true);
    setStatus(null);
    const result = await onSubmit(text);
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
          {busy ? "Inserting…" : "Insert all"}
        </button>
      </div>
    </div>
  );
}
