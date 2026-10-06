"use client";

import { useCallback, useMemo, useState } from "react";

export type BulkSelection<T> = {
  selected: ReadonlySet<string>;
  count: number;
  isSelected: (id: string) => boolean;
  toggle: (id: string) => void;
  toggleAll: () => void;
  clear: () => void;
  allSelected: boolean;
  someSelected: boolean;
  selectedRows: ReadonlyArray<T>;
};

export function useBulkSelection<T extends { id: string }>(
  rows: ReadonlyArray<T>,
): BulkSelection<T> {
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [prevRows, setPrevRows] = useState(rows);

  // Drop ids that no longer exist in the current row set (e.g. after a refresh
  // or delete). Adjusted during render rather than in an effect, so the stale
  // selection is never committed.
  if (rows !== prevRows) {
    setPrevRows(rows);
    if (selected.size > 0) {
      const valid = new Set(rows.map((r) => r.id));
      const next = new Set([...selected].filter((id) => valid.has(id)));
      if (next.size !== selected.size) setSelected(next);
    }
  }

  const toggle = useCallback((id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const allSelected = rows.length > 0 && rows.every((r) => selected.has(r.id));
  const someSelected = !allSelected && rows.some((r) => selected.has(r.id));

  const toggleAll = useCallback(() => {
    setSelected((prev) => {
      const allInRows = rows.length > 0 && rows.every((r) => prev.has(r.id));
      const next = new Set(prev);
      if (allInRows) {
        for (const r of rows) next.delete(r.id);
      } else {
        for (const r of rows) next.add(r.id);
      }
      return next;
    });
  }, [rows]);

  const clear = useCallback(() => setSelected(new Set()), []);

  const isSelected = useCallback((id: string) => selected.has(id), [selected]);

  const selectedRows = useMemo(() => rows.filter((r) => selected.has(r.id)), [rows, selected]);

  return {
    selected,
    count: selected.size,
    isSelected,
    toggle,
    toggleAll,
    clear,
    allSelected,
    someSelected,
    selectedRows,
  };
}
