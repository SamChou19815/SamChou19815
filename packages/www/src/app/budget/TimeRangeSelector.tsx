"use client";

import { useEffect, useState } from "react";
import { endOfCurrentMonth, parseLocalDate, todayISO } from "./utils";

export type RangePreset = "MONTH" | "3M" | "6M" | "12M" | "YTD" | "ALL" | "CUSTOM";

export type TimeRange = {
  start: Date;
  end: Date;
  preset: RangePreset;
};

const PRESET_LABELS: Record<RangePreset, string> = {
  MONTH: "Month",
  "3M": "3M",
  "6M": "6M",
  "12M": "12M",
  YTD: "YTD",
  ALL: "All",
  CUSTOM: "Custom",
};

const DEFAULT_PRESETS: ReadonlyArray<RangePreset> = ["3M", "6M", "12M", "YTD", "ALL", "CUSTOM"];

function startForPreset(preset: RangePreset, end: Date): Date {
  if (preset === "ALL") return new Date(2000, 0, 1);
  if (preset === "YTD") return new Date(end.getFullYear(), 0, 1);
  if (preset === "MONTH") return new Date(end.getFullYear(), end.getMonth(), 1);
  const months = preset === "3M" ? 3 : preset === "6M" ? 6 : 12;
  const d = new Date(end.getFullYear(), end.getMonth() - (months - 1), 1);
  return d;
}

export function rangeFromPreset(preset: Exclude<RangePreset, "CUSTOM">): TimeRange {
  const end = preset === "MONTH" ? endOfCurrentMonth() : new Date();
  return { start: startForPreset(preset, end), end, preset };
}

export function presetLabel(preset: RangePreset): string {
  if (preset === "MONTH") return "this month";
  if (preset === "ALL") return "all";
  if (preset === "CUSTOM") return "custom";
  return preset;
}

export default function TimeRangeSelector({
  value,
  onChange,
  presets = DEFAULT_PRESETS,
}: {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  presets?: ReadonlyArray<RangePreset>;
}): React.JSX.Element {
  const [customStart, setCustomStart] = useState<string>(todayISO());
  const [customEnd, setCustomEnd] = useState<string>(todayISO());

  useEffect(() => {
    if (value.preset === "CUSTOM") {
      setCustomStart(toISODate(value.start));
      setCustomEnd(toISODate(value.end));
    }
  }, [value]);

  const selectPreset = (preset: RangePreset) => {
    if (preset === "CUSTOM") {
      onChange({
        start: parseLocalDate(customStart),
        end: parseLocalDate(customEnd),
        preset: "CUSTOM",
      });
      return;
    }
    onChange(rangeFromPreset(preset));
  };

  return (
    <div className="flex flex-wrap items-center gap-2">
      {presets.map((id) => {
        const active = id === value.preset;
        const cls = active
          ? "rounded bg-blue-500 px-3 py-1 text-sm font-medium text-white dark:bg-blue-400 dark:text-gray-900"
          : "rounded border border-gray-300 px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";
        return (
          <button key={id} type="button" onClick={() => selectPreset(id)} className={cls}>
            {PRESET_LABELS[id]}
          </button>
        );
      })}
      {value.preset === "CUSTOM" && (
        <div className="flex items-center gap-2">
          <input
            type="date"
            value={customStart}
            onChange={(e) => {
              setCustomStart(e.target.value);
              onChange({
                start: parseLocalDate(e.target.value),
                end: parseLocalDate(customEnd),
                preset: "CUSTOM",
              });
            }}
            className="rounded border border-gray-300 bg-white px-2 py-1 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          />
          <span className="text-gray-500">→</span>
          <input
            type="date"
            value={customEnd}
            onChange={(e) => {
              setCustomEnd(e.target.value);
              onChange({
                start: parseLocalDate(customStart),
                end: parseLocalDate(e.target.value),
                preset: "CUSTOM",
              });
            }}
            className="rounded border border-gray-300 bg-white px-2 py-1 text-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          />
        </div>
      )}
    </div>
  );
}

function toISODate(d: Date): string {
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

export function defaultRange(): TimeRange {
  return rangeFromPreset("12M");
}
