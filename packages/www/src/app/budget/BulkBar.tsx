"use client";

import { useState, type ReactNode } from "react";

type Props = {
  count: number;
  onDelete: () => Promise<void> | void;
  onClear: () => void;
  editForm: ReactNode;
  onApplyEdit: () => Promise<void> | void;
  itemNoun?: string;
};

export default function BulkBar({
  count,
  onDelete,
  onClear,
  editForm,
  onApplyEdit,
  itemNoun = "row",
}: Props): React.JSX.Element | null {
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  if (count === 0) return null;

  const noun = count === 1 ? itemNoun : `${itemNoun}s`;

  const doDelete = async () => {
    if (!window.confirm(`Delete ${count} selected ${noun}?`)) return;
    setBusy(true);
    try {
      await onDelete();
    } finally {
      setBusy(false);
    }
  };

  const doApply = async () => {
    setBusy(true);
    try {
      await onApplyEdit();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mb-3 rounded border border-blue-300 bg-blue-50 px-3 py-2 dark:border-blue-700 dark:bg-blue-900/20">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-medium text-blue-700 dark:text-blue-300">
          {count} selected
        </span>
        <div className="ml-auto flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => setEditing((v) => !v)}
            disabled={busy}
            className="rounded border border-blue-500 px-3 py-1 text-sm font-medium text-blue-600 hover:bg-blue-100 disabled:opacity-50 dark:border-blue-400 dark:text-blue-300 dark:hover:bg-blue-900/40"
          >
            {editing ? "Hide edit" : "Edit fields"}
          </button>
          <button
            type="button"
            onClick={doDelete}
            disabled={busy}
            className="rounded bg-red-500 px-3 py-1 text-sm font-bold text-white hover:bg-red-600 disabled:opacity-50"
          >
            Delete selected
          </button>
          <button
            type="button"
            onClick={onClear}
            disabled={busy}
            className="rounded border border-gray-300 px-3 py-1 text-sm font-medium hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
          >
            Clear
          </button>
        </div>
      </div>
      {editing && (
        <div className="mt-3 border-t border-blue-200 pt-3 dark:border-blue-700">
          <p className="mb-2 text-xs text-gray-600 dark:text-gray-400">
            Tick a checkbox to apply that field to all {count} selected {noun}. Unticked fields stay
            unchanged.
          </p>
          {editForm}
          <div className="mt-3 flex justify-end">
            <button
              type="button"
              onClick={doApply}
              disabled={busy}
              className="rounded bg-blue-500 px-4 py-1 text-sm font-bold text-white hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
            >
              {busy ? "Applying…" : `Apply to ${count} ${noun}`}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export function BulkEditField({
  label,
  set,
  onSet,
  children,
  className,
}: {
  label: string;
  set: boolean;
  onSet: (v: boolean) => void;
  children: ReactNode;
  className?: string;
}): React.JSX.Element {
  return (
    <div className={`flex flex-col text-sm ${className ?? ""}`}>
      <label className="mb-1 flex items-center gap-2 text-gray-600 dark:text-gray-400">
        <input
          type="checkbox"
          checked={set}
          onChange={(e) => onSet(e.target.checked)}
          className="h-3 w-3"
        />
        <span>{label}</span>
      </label>
      <div className={set ? "" : "opacity-50"}>{children}</div>
    </div>
  );
}

export function HeaderCheckbox({
  allSelected,
  someSelected,
  onToggle,
}: {
  allSelected: boolean;
  someSelected: boolean;
  onToggle: () => void;
}): React.JSX.Element {
  return (
    <input
      type="checkbox"
      aria-label="Select all"
      checked={allSelected}
      ref={(el) => {
        if (el) el.indeterminate = !allSelected && someSelected;
      }}
      onChange={onToggle}
      className="h-4 w-4"
    />
  );
}

export function RowCheckbox({
  checked,
  onToggle,
}: {
  checked: boolean;
  onToggle: () => void;
}): React.JSX.Element {
  return (
    <input
      type="checkbox"
      aria-label="Select row"
      checked={checked}
      onChange={onToggle}
      className="h-4 w-4"
    />
  );
}
