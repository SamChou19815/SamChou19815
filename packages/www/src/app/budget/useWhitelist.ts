"use client";

import { useEffect, useState } from "react";
import { getSupabase } from "../../lib/supabase";

export type WhitelistResult = "loading" | "allowed" | "denied";

export function useWhitelist(email: string | null | undefined): WhitelistResult {
  // Tagged with the email it was checked for, so a stale answer reads as
  // "loading" instead of being reset synchronously when the email changes.
  const [checked, setChecked] = useState<{
    email: string;
    result: Exclude<WhitelistResult, "loading">;
  } | null>(null);

  useEffect(() => {
    if (email == null) return;
    let cancelled = false;
    getSupabase()
      .from("allowed_users")
      .select("email")
      .eq("email", email)
      .limit(1)
      .then(({ data, error }) => {
        if (cancelled) return;
        if (error != null) {
          setChecked({ email, result: "denied" });
          return;
        }
        setChecked({ email, result: data != null && data.length > 0 ? "allowed" : "denied" });
      });
    return () => {
      cancelled = true;
    };
  }, [email]);

  return email != null && checked?.email === email ? checked.result : "loading";
}
