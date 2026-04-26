"use client";

import { useCallback, useEffect, useMemo, useState } from "react";

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

  // Drop ids that no longer exist in the current row set (e.g. after a refresh or delete).
  useEffect(() => {
    setSelected((prev) => {
      if (prev.size === 0) return prev;
      const valid = new Set(rows.map((r) => r.id));
      let changed = false;
      const next = new Set<string>();
      for (const id of prev) {
        if (valid.has(id)) next.add(id);
        else changed = true;
      }
      return changed ? next : prev;
    });
  }, [rows]);

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
