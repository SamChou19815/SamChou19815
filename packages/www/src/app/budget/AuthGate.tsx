"use client";

import { useState, type ReactNode } from "react";
import { getSupabase, supabaseConfigured } from "../../lib/supabase";
import { Card } from "./BudgetApp";
import { useAuth } from "./useAuth";
import { useWhitelist } from "./useWhitelist";

function PageWrapper({ children }: { children: ReactNode }): React.JSX.Element {
  return <div className="mx-auto max-w-6xl px-4 py-12">{children}</div>;
}

type Mode = "signIn" | "signUp";

function EmailAuthForm(): React.JSX.Element {
  const [mode, setMode] = useState<Mode>("signIn");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setInfo(null);
    if (email.trim() === "" || password === "") {
      setError("Email and password are required.");
      return;
    }
    setSubmitting(true);
    const supabase = getSupabase();
    if (mode === "signIn") {
      const { error: err } = await supabase.auth.signInWithPassword({
        email: email.trim(),
        password,
      });
      setSubmitting(false);
      if (err != null) setError(err.message);
    } else {
      const { data, error: err } = await supabase.auth.signUp({
        email: email.trim(),
        password,
        options: { emailRedirectTo: `${window.location.origin}/budget` },
      });
      setSubmitting(false);
      if (err != null) {
        setError(err.message);
        return;
      }
      if (data.session == null) {
        setInfo("Check your inbox for a confirmation link.");
      }
    }
  };

  const inputCls =
    "w-full rounded border border-gray-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";

  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3">
      {error && (
        <div className="rounded border border-red-400 bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
          {error}
        </div>
      )}
      {info && (
        <div className="rounded border border-blue-400 bg-blue-50 px-3 py-2 text-sm text-blue-700 dark:bg-blue-900/20 dark:text-blue-300">
          {info}
        </div>
      )}
      <label className="flex flex-col text-sm">
        <span className="mb-1 text-gray-600 dark:text-gray-400">Email</span>
        <input
          type="email"
          autoComplete="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          className={inputCls}
        />
      </label>
      <label className="flex flex-col text-sm">
        <span className="mb-1 text-gray-600 dark:text-gray-400">Password</span>
        <input
          type="password"
          autoComplete={mode === "signIn" ? "current-password" : "new-password"}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className={inputCls}
        />
      </label>
      <button
        type="submit"
        disabled={submitting}
        className="rounded bg-blue-500 px-6 py-2 font-bold text-sm text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
      >
        {submitting ? "…" : mode === "signIn" ? "Sign in" : "Sign up"}
      </button>
      <button
        type="button"
        onClick={() => {
          setMode((m) => (m === "signIn" ? "signUp" : "signIn"));
          setError(null);
          setInfo(null);
        }}
        className="text-center text-sm text-blue-500 hover:underline dark:text-blue-400"
      >
        {mode === "signIn" ? "Create an account" : "Already have an account? Sign in"}
      </button>
    </form>
  );
}

export default function AuthGate({ children }: { children: ReactNode }): React.JSX.Element {
  const { session, status } = useAuth();
  const email = session?.user.email ?? null;
  const whitelist = useWhitelist(status === "signedIn" ? email : null);

  if (!supabaseConfigured) {
    return (
      <PageWrapper>
        <Card>
          <h1 className="mb-2">Budget</h1>
          <p className="text-gray-500 dark:text-gray-400">
            Supabase isn&apos;t configured for this build. Set <code>NEXT_PUBLIC_SUPABASE_URL</code>{" "}
            and <code>NEXT_PUBLIC_SUPABASE_ANON_KEY</code> in <code>packages/www/.env.local</code>{" "}
            and rebuild.
          </p>
        </Card>
      </PageWrapper>
    );
  }

  if (status === "loading") {
    return (
      <PageWrapper>
        <Card>
          <div className="text-center text-gray-500 dark:text-gray-400">Loading session…</div>
        </Card>
      </PageWrapper>
    );
  }

  if (status === "signedOut") {
    return (
      <PageWrapper>
        <div className="mx-auto max-w-md">
          <Card>
            <h1 className="text-center mb-2">Budget</h1>
            <p className="text-center text-gray-500 mb-6 dark:text-gray-400">
              Sign in to view your dashboard.
            </p>
            <EmailAuthForm />
          </Card>
        </div>
      </PageWrapper>
    );
  }

  if (whitelist === "loading") {
    return (
      <PageWrapper>
        <Card>
          <div className="text-center text-gray-500 dark:text-gray-400">Checking access…</div>
        </Card>
      </PageWrapper>
    );
  }

  if (whitelist === "denied") {
    return (
      <PageWrapper>
        <Card>
          <h1 className="text-center mb-2">Access denied</h1>
          <p className="text-center text-gray-500 mb-6 dark:text-gray-400">
            Access denied — your account is not on the whitelist.
          </p>
          <div className="flex justify-center">
            <button
              type="button"
              onClick={() => getSupabase().auth.signOut()}
              className="rounded border border-gray-300 px-6 py-2 font-bold text-sm text-gray-700 transition-colors duration-200 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
            >
              Sign out
            </button>
          </div>
        </Card>
      </PageWrapper>
    );
  }

  return <>{children}</>;
}
