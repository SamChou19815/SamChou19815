"use client";

import { useEffect, useState } from "react";
import { getSupabase } from "../../lib/supabase";

type WithId = { id: string };

type Options<T> = {
  table: string;
  userId: string | null | undefined;
  orderBy: { column: keyof T & string; ascending: boolean };
};

export type RealtimeRowsState<T> = {
  rows: T[];
  loading: boolean;
  error: string | null;
};

export function useRealtimeRows<T extends WithId>({
  table,
  userId,
  orderBy,
}: Options<T>): RealtimeRowsState<T> {
  const [rows, setRows] = useState<T[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (userId == null) {
      setLoading(false);
      setRows([]);
      return;
    }
    const supabase = getSupabase();
    let cancelled = false;
    setLoading(true);
    setError(null);

    const sortRows = (input: T[]): T[] => {
      const dir = orderBy.ascending ? 1 : -1;
      return input.slice().sort((a, b) => {
        const av = a[orderBy.column];
        const bv = b[orderBy.column];
        if (av == null && bv == null) return 0;
        if (av == null) return -1 * dir;
        if (bv == null) return 1 * dir;
        if (av < bv) return -1 * dir;
        if (av > bv) return 1 * dir;
        return 0;
      });
    };

    supabase
      .from(table)
      .select("*")
      .eq("user_id", userId)
      .order(orderBy.column, { ascending: orderBy.ascending })
      .then(({ data, error: err }) => {
        if (cancelled) return;
        if (err != null) {
          setError(err.message);
          setRows([]);
          setLoading(false);
          return;
        }
        setRows(sortRows((data ?? []) as T[]));
        setLoading(false);
      });

    const filter = `user_id=eq.${userId}`;
    const channel = supabase
      .channel(`rt:${table}:${userId}`)
      .on(
        // oxlint-disable-next-line no-explicit-any
        "postgres_changes" as any,
        { event: "*", schema: "public", table, filter },
        // oxlint-disable-next-line no-explicit-any
        (payload: any) => {
          if (cancelled) return;
          if (payload.eventType === "INSERT") {
            const next = payload.new as T;
            setRows((prev) =>
              sortRows(prev.some((r) => r.id === next.id) ? prev : [...prev, next]),
            );
          } else if (payload.eventType === "UPDATE") {
            const next = payload.new as T;
            setRows((prev) => sortRows(prev.map((r) => (r.id === next.id ? next : r))));
          } else if (payload.eventType === "DELETE") {
            const old = payload.old as T;
            setRows((prev) => prev.filter((r) => r.id !== old.id));
          }
        },
      )
      .subscribe();

    return () => {
      cancelled = true;
      supabase.removeChannel(channel);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [table, userId, orderBy.column, orderBy.ascending]);

  return { rows, loading, error };
}
