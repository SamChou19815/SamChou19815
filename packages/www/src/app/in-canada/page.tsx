"use client";

import { useEffect, useState } from "react";
import NavBar from "../../lib/NavBar";

const START_DATE = new Date("2025-01-24");

const MISSING_DAYS: string[] = process.env.NEXT_PUBLIC_IN_CANADA_MISSING_DAYS
  ? process.env.NEXT_PUBLIC_IN_CANADA_MISSING_DAYS.split("\n").map((d) => d.trim())
  : [];

function getDaysBetween(start: Date, end: Date): number {
  const msPerDay = 24 * 60 * 60 * 1000;
  return Math.floor((end.getTime() - start.getTime()) / msPerDay) + 1;
}

function countDaysInCanada(today: Date): {
  totalDays: number;
  daysInCanada: number;
  missingDaysCount: number;
} {
  const totalDays = getDaysBetween(START_DATE, today);
  const missingDaysInRange = MISSING_DAYS.filter((dateStr) => {
    const date = new Date(dateStr);
    return date >= START_DATE && date <= today;
  });
  const missingDaysCount = missingDaysInRange.length;
  const daysInCanada = totalDays - missingDaysCount;
  return { totalDays, daysInCanada, missingDaysCount };
}

function Card({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col bg-white rounded filter drop-shadow hover:drop-shadow-lg transition-all duration-300 ease-out p-6">
      {children}
    </div>
  );
}

function StatBox({ label, value }: { label: string; value: number | string }): React.JSX.Element {
  return (
    <div className="text-center">
      <div className="text-4xl font-bold text-blue-500">{value}</div>
      <div className="text-gray-500 mt-1">{label}</div>
    </div>
  );
}

const MISSING_DAYS_SET = new Set(MISSING_DAYS);

function getMonthsInRange(start: Date, end: Date): { year: number; month: number }[] {
  const months: { year: number; month: number }[] = [];
  const current = new Date(start.getFullYear(), start.getMonth(), 1);
  const endMonth = new Date(end.getFullYear(), end.getMonth(), 1);

  while (current <= endMonth) {
    months.push({ year: current.getFullYear(), month: current.getMonth() });
    current.setMonth(current.getMonth() + 1);
  }
  return months;
}

function getDaysInMonth(year: number, month: number): number {
  return new Date(year, month + 1, 0).getDate();
}

function getFirstDayOfMonth(year: number, month: number): number {
  return new Date(year, month, 1).getDay();
}

function formatDateString(year: number, month: number, day: number): string {
  return `${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function MonthCalendar({
  year,
  month,
  today,
}: { year: number; month: number; today: Date }): React.JSX.Element {
  const daysInMonth = getDaysInMonth(year, month);
  const firstDay = getFirstDayOfMonth(year, month);
  const monthName = new Date(year, month).toLocaleDateString("en-US", { month: "short" });

  const days: React.JSX.Element[] = [];

  // Empty cells for days before the first day of the month
  for (let i = 0; i < firstDay; i++) {
    days.push(<div key={`empty-${i}`} className="w-5 h-5" />);
  }

  // Days of the month
  for (let day = 1; day <= daysInMonth; day++) {
    const dateStr = formatDateString(year, month, day);
    const date = new Date(year, month, day);
    const isMissing = MISSING_DAYS_SET.has(dateStr);
    const isBeforeStart = date < START_DATE;
    const isAfterToday = date > today;
    const isOutOfRange = isBeforeStart || isAfterToday;

    let className = "w-5 h-5 text-xs flex items-center justify-center rounded ";
    if (isOutOfRange) {
      className += "text-gray-300";
    } else if (isMissing) {
      className += "bg-red-400 text-white font-medium";
    } else {
      className += "bg-green-400 text-white";
    }

    days.push(
      <div key={day} className={className}>
        {day}
      </div>,
    );
  }

  return (
    <div className="flex flex-col items-center">
      <div className="text-xs font-medium text-gray-600 mb-1">
        {monthName} {year}
      </div>
      <div className="grid grid-cols-7 gap-0.5">{days}</div>
    </div>
  );
}

function Calendar({ today }: { today: Date }): React.JSX.Element {
  const months = getMonthsInRange(START_DATE, today);

  return (
    <div className="border-t pt-4 mt-4">
      <h4 className="text-gray-700 mb-4">Calendar View</h4>
      <div className="flex items-center gap-4 mb-4 text-xs">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-green-400" />
          <span className="text-gray-500">In Canada</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-red-400" />
          <span className="text-gray-500">Outside Canada</span>
        </div>
      </div>
      <div className="flex flex-wrap gap-4 justify-center">
        {months.map(({ year, month }) => (
          <MonthCalendar key={`${year}-${month}`} year={year} month={month} today={today} />
        ))}
      </div>
    </div>
  );
}

export default function CanadaPage(): React.JSX.Element {
  const [today, setToday] = useState<Date | null>(null);

  useEffect(() => {
    setToday(new Date());
  }, []);

  const stats = today ? countDaysInCanada(today) : null;
  const startDateFormatted = START_DATE.toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  return (
    <>
      <NavBar title="Developer Sam" titleLink="/" navItems={[{ name: "Blog", link: "/blog" }]} />
      <div className="mx-auto max-w-4xl px-4 py-12">
        <Card>
          <h1 className="text-center mb-2">In-Canada Days Counter</h1>
          <p className="text-center text-gray-500 mb-8">Counting since {startDateFormatted}</p>

          <div className="flex flex-row justify-around mb-8">
            <StatBox label="Days in Canada" value={stats?.daysInCanada ?? "—"} />
            <StatBox label="Days Away" value={stats?.missingDaysCount ?? "—"} />
            <StatBox label="Total Days" value={stats?.totalDays ?? "—"} />
          </div>

          {today && <Calendar today={today} />}
        </Card>
      </div>
    </>
  );
}
