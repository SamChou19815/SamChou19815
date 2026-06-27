import { createClient, type SupabaseClient } from "@supabase/supabase-js";

const url = "https://yrklwvnpkqhmhsmmaele.supabase.co";
const anonKey = "sb_publishable_cWyX02iHRP9tHRaFwyf8Wg_EXlZKv3I";

let client: SupabaseClient | null = null;

export function getSupabase(): SupabaseClient {
  if (client == null) {
    client = createClient(url as string, anonKey as string, {
      auth: {
        persistSession: true,
        autoRefreshToken: true,
        detectSessionInUrl: true,
      },
    });
  }
  return client;
}
