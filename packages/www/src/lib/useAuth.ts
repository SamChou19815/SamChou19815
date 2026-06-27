"use client";

import { useEffect, useState } from "react";
import type { Session } from "@supabase/supabase-js";
import { getSupabase } from "./supabase";

export type AuthStatus = "loading" | "signedOut" | "signedIn";

export type AuthState = {
  session: Session | null;
  status: AuthStatus;
};

export function useAuth(): AuthState {
  const [state, setState] = useState<AuthState>({
    session: null,
    status: "loading",
  });

  useEffect(() => {
    const supabase = getSupabase();
    let mounted = true;

    supabase.auth.getSession().then(({ data }) => {
      if (!mounted) return;
      setState({
        session: data.session,
        status: data.session != null ? "signedIn" : "signedOut",
      });
    });

    const { data: sub } = supabase.auth.onAuthStateChange((_event, session) => {
      setState({
        session,
        status: session != null ? "signedIn" : "signedOut",
      });
    });

    return () => {
      mounted = false;
      sub.subscription.unsubscribe();
    };
  }, []);

  return state;
}
