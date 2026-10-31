import { getSupabase } from "../../lib/supabase";

// The one table every app under /random-small-apps shares: a single row per
// (user, app) holding that app's whole state as one JSON document, assumed
// small enough to read and write in one piece. Create it once in the Supabase
// SQL editor:
//
//   create table if not exists public.random_small_apps (
//     user_id uuid not null references auth.users (id) on delete cascade,
//     app_id text not null,
//     data jsonb not null,
//     updated_at timestamptz not null default now(),
//     primary key (user_id, app_id)
//   );
//
//   -- Table privileges for the API roles. Without this grant every request
//   -- fails with `permission denied for table random_small_apps`, no matter
//   -- what the RLS policies say.
//   grant select, insert, update, delete on public.random_small_apps
//     to anon, authenticated;
//
//   alter table public.random_small_apps enable row level security;
//
//   create policy "select own rows" on public.random_small_apps
//     for select to authenticated using (auth.uid() = user_id);
//   create policy "insert own rows" on public.random_small_apps
//     for insert to authenticated with check (auth.uid() = user_id);
//   create policy "update own rows" on public.random_small_apps
//     for update to authenticated using (auth.uid() = user_id)
//     with check (auth.uid() = user_id);
//   create policy "delete own rows" on public.random_small_apps
//     for delete to authenticated using (auth.uid() = user_id);

/**
 * Loads the app's JSON document for the user, or `null` when there is no row
 * yet — an app's empty state is its own business, not the table's.
 */
export async function loadSmallAppData(
  userId: string,
  appId: string,
): Promise<{ data: unknown | null; error: string | null }> {
  const { data, error } = await getSupabase()
    .from("random_small_apps")
    .select("data")
    .eq("user_id", userId)
    .eq("app_id", appId)
    .maybeSingle();
  if (error != null) return { data: null, error: error.message };
  return { data: data?.data ?? null, error: null };
}

/** Saves the app's JSON document for the user, creating the row on first save. */
export async function saveSmallAppData(
  userId: string,
  appId: string,
  data: unknown,
): Promise<string | null> {
  const { error } = await getSupabase()
    .from("random_small_apps")
    .upsert(
      { user_id: userId, app_id: appId, data, updated_at: new Date().toISOString() },
      { onConflict: "user_id,app_id" },
    );
  return error != null ? error.message : null;
}
