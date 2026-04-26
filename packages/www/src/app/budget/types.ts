export type Expense = {
  id: string;
  user_id: string;
  name: string;
  category: string;
  amount: number;
  date: string;
  notes: string | null;
  created_at: string;
};

export type Income = {
  id: string;
  user_id: string;
  name: string;
  source: string | null;
  amount: number;
  date: string;
  notes: string | null;
  created_at: string;
};

export type Currency = "CAD" | "USD";

export type Investment = {
  id: string;
  user_id: string;
  name: string;
  type: string;
  currency: Currency;
  total_cost: number;
  total_market_value: number;
  exchange_rate: number;
  updated_at: string;
  created_at: string;
};

export type InvestmentSnapshot = {
  id: string;
  user_id: string;
  investment_id: string;
  total_cost: number;
  total_market_value: number;
  exchange_rate: number;
  recorded_at: string;
  created_at: string;
};

export type TabId = "dashboard" | "expenses" | "income" | "investments" | "fire";

export const TAB_IDS: ReadonlyArray<TabId> = [
  "dashboard",
  "expenses",
  "income",
  "investments",
  "fire",
];

export function isTabId(value: string | null): value is TabId {
  return value != null && (TAB_IDS as readonly string[]).includes(value);
}
