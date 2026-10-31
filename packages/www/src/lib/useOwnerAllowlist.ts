import type { AccessResult } from "./AuthGate";

// The personal tools that don't keep their own allowlist in the database are
// restricted to this single account.
const OWNER_EMAIL = "sam@developersam.com";

export function useOwnerAllowlist(email: string | null): AccessResult {
  if (email == null) return "loading";
  return email === OWNER_EMAIL ? "allowed" : "denied";
}
