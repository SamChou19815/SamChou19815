"use client";

import type { AccessResult } from "../../lib/AuthGate";

// The in-canada app is restricted to this single account.
const OWNER_EMAIL = "sam@developersam.com";

export function useOwnerAllowlist(email: string | null): AccessResult {
  if (email == null) return "loading";
  return email === OWNER_EMAIL ? "allowed" : "denied";
}
