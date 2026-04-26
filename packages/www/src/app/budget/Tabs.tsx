"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { isTabId, TAB_IDS, type TabId } from "./types";

const TAB_LABELS: Record<TabId, string> = {
  dashboard: "Dashboard",
  expenses: "Expenses",
  income: "Income",
  investments: "Investments",
  fire: "FIRE",
};

export function useActiveTab(): TabId {
  const params = useSearchParams();
  const raw = params.get("tab");
  return isTabId(raw) ? raw : "dashboard";
}

export default function Tabs(): React.JSX.Element {
  const router = useRouter();
  const params = useSearchParams();
  const active = useActiveTab();

  const onSelect = (id: TabId) => {
    const next = new URLSearchParams(params.toString());
    if (id === "dashboard") {
      next.delete("tab");
    } else {
      next.set("tab", id);
    }
    const qs = next.toString();
    router.replace(qs.length > 0 ? `/budget?${qs}` : "/budget");
  };

  return (
    <div className="overflow-x-auto">
      <div className="flex gap-2 border-b border-gray-200 dark:border-gray-700">
        {TAB_IDS.map((id) => {
          const isActive = id === active;
          const base =
            "whitespace-nowrap px-4 py-2 text-sm font-medium transition-colors duration-200 border-b-2 -mb-px";
          const cls = isActive
            ? `${base} border-blue-500 text-blue-500 dark:border-blue-400 dark:text-blue-400`
            : `${base} border-transparent text-gray-600 hover:text-blue-500 dark:text-gray-300 dark:hover:text-blue-400`;
          return (
            <button
              type="button"
              key={id}
              className={cls}
              onClick={() => onSelect(id)}
              aria-current={isActive ? "page" : undefined}
            >
              {TAB_LABELS[id]}
            </button>
          );
        })}
      </div>
    </div>
  );
}
