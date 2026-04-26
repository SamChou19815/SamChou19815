"use client";

import { useEffect, useState } from "react";
import { getSupabase, supabaseConfigured } from "../../lib/supabase";

export type WhitelistResult = "loading" | "allowed" | "denied";

export function useWhitelist(email: string | null | undefined): WhitelistResult {
  const [result, setResult] = useState<WhitelistResult>("loading");

  useEffect(() => {
    if (!supabaseConfigured || email == null) {
      setResult("loading");
      return;
    }
    let cancelled = false;
    setResult("loading");
    getSupabase()
      .from("allowed_users")
      .select("email")
      .eq("email", email)
      .limit(1)
      .then(({ data, error }) => {
        if (cancelled) return;
        if (error != null) {
          setResult("denied");
          return;
        }
        setResult(data != null && data.length > 0 ? "allowed" : "denied");
      });
    return () => {
      cancelled = true;
    };
  }, [email]);

  return result;
}
