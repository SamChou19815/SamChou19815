import { useEffect, useMemo, useState } from "react";
import { useAuth } from "../../../lib/useAuth";
import { RANDOM_SMALL_APPS } from "../apps";
import { loadSmallAppData, saveSmallAppData } from "../smallAppStorage";

// The apps whose documents this editor can edit: everything but itself.
const EDITABLE_APPS = RANDOM_SMALL_APPS.filter(({ id }) => id !== "edit-json");

const inputCls =
  "rounded border border-gray-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100";

const primaryButtonCls =
  "rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300";

const secondaryButtonCls =
  "rounded border border-gray-300 px-4 py-2 text-sm font-bold text-gray-700 transition-colors duration-200 hover:bg-gray-100 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";

function Card({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col bg-white transition-all duration-300 ease-out p-6 dark:bg-[#242424] max-sm:-mx-6 sm:rounded sm:filter sm:drop-shadow sm:hover:drop-shadow-lg sm:dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] sm:dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
      {children}
    </div>
  );
}

/** Pretty-print exactly the way it is stored and reloaded. */
function toEditableText(data: unknown): string {
  return `${JSON.stringify(data ?? {}, null, 2)}\n`;
}

type LoadState = "loading" | "ready" | "error";

export default function JsonEditorApp(): React.JSX.Element {
  const { session } = useAuth();
  const userId = session?.user.id ?? null;

  const [appId, setAppId] = useState(EDITABLE_APPS[0]?.id ?? "");
  const [text, setText] = useState("");
  // Tagged with the document it was loaded for, so switching apps or accounts
  // reads as "loading" instead of showing the previous document.
  const [loaded, setLoaded] = useState<{
    key: string;
    state: Exclude<LoadState, "loading">;
  } | null>(null);
  const loadKey = userId != null ? `${userId}:${appId}` : null;
  const loadState: LoadState =
    loadKey != null && loaded?.key === loadKey ? loaded.state : "loading";
  const [reloadNonce, setReloadNonce] = useState(0);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  useEffect(() => {
    if (userId == null || appId === "") return;
    let cancelled = false;
    loadSmallAppData(userId, appId).then(({ data, error }) => {
      if (cancelled) return;
      if (error != null) {
        setLoaded({ key: `${userId}:${appId}`, state: "error" });
        setMessage({ kind: "error", text: error });
        return;
      }
      setText(toEditableText(data));
      setLoaded({ key: `${userId}:${appId}`, state: "ready" });
    });
    return () => {
      cancelled = true;
    };
    // `reloadNonce` is deliberately not read here: the Reload button bumps it
    // to run this effect again.
    // oxlint-disable-next-line exhaustive-effect-dependencies
  }, [userId, appId, reloadNonce]);

  useEffect(() => {
    if (message?.kind !== "ok") return;
    const t = setTimeout(() => setMessage(null), 4000);
    return () => clearTimeout(t);
  }, [message]);

  // Re-parsed on every keystroke; only valid JSON can be saved.
  const parsed = useMemo<{ ok: boolean; error: string | null }>(() => {
    try {
      JSON.parse(text);
      return { ok: true, error: null };
    } catch (e) {
      return { ok: false, error: e instanceof Error ? e.message : String(e) };
    }
  }, [text]);

  const save = async () => {
    if (userId == null || appId === "" || !parsed.ok || saving) return;
    setSaving(true);
    setMessage(null);
    const error = await saveSmallAppData(userId, appId, JSON.parse(text));
    setSaving(false);
    if (error != null) {
      setMessage({ kind: "error", text: error });
      return;
    }
    setMessage({ kind: "ok", text: "Saved." });
  };

  const reload = () => {
    setMessage(null);
    setLoaded(null);
    setReloadNonce((n) => n + 1);
  };

  const format = () => {
    if (!parsed.ok) return;
    setText(toEditableText(JSON.parse(text)));
  };

  if (EDITABLE_APPS.length === 0) {
    return (
      <div className="mx-auto w-full max-w-6xl px-6 py-12">
        <Card>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No apps to edit yet — register one in apps.ts first.
          </p>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <section className="bg-slate-900 text-white">
        <div className="mx-auto max-w-6xl px-6 pt-12 pb-10">
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
            Random Small Apps
          </p>
          <h1 className="mt-2 font-sans text-3xl font-bold tracking-tight text-white">
            JSON Editor
          </h1>
          <p className="mt-3 max-w-2xl text-sm text-slate-300">
            Edit an app&apos;s saved document as raw JSON — an escape hatch for bulk fixes the
            app&apos;s own UI can&apos;t express. Editing is on you: apps tolerate unknown shapes on
            load by dropping what they don&apos;t recognize, so a careless edit can silently lose
            data.
          </p>
        </div>
      </section>

      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-10">
        <Card>
          <label className="flex flex-col text-sm">
            <span className="mb-1 text-gray-600 dark:text-gray-400">App</span>
            <select
              value={appId}
              onChange={(e) => {
                setAppId(e.target.value);
                setMessage(null);
              }}
              className={`${inputCls} w-64`}
            >
              {EDITABLE_APPS.map((app) => (
                <option key={app.id} value={app.id}>
                  {app.name}
                </option>
              ))}
            </select>
          </label>

          {loadState === "loading" ? (
            <div className="mt-4 text-sm text-gray-500 dark:text-gray-400">Loading…</div>
          ) : loadState === "error" ? (
            <div className="mt-4 flex flex-wrap items-center gap-4">
              <p className="text-sm text-red-600 dark:text-red-400">
                {message?.text ?? "Unknown error."}
              </p>
              <button type="button" onClick={reload} className={secondaryButtonCls}>
                Retry
              </button>
            </div>
          ) : (
            <>
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                spellCheck={false}
                aria-label="JSON document"
                rows={20}
                className="mt-4 w-full rounded border border-gray-300 bg-white px-3 py-2 font-mono text-sm focus:border-blue-500 focus:outline-none disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
              />
              {!parsed.ok && (
                <p className="mt-2 font-mono text-sm text-red-600 dark:text-red-400">
                  {`Invalid JSON — ${parsed.error}`}
                </p>
              )}
              {message != null && (
                <div
                  className={`mt-4 rounded border px-3 py-2 text-sm ${
                    message.kind === "ok"
                      ? "border-blue-400 bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300"
                      : "border-red-400 bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"
                  }`}
                >
                  {message.text}
                </div>
              )}
              <div className="mt-4 flex flex-wrap justify-end gap-3">
                <button
                  type="button"
                  onClick={format}
                  disabled={!parsed.ok}
                  className={secondaryButtonCls}
                >
                  Format
                </button>
                <button type="button" onClick={reload} className={secondaryButtonCls}>
                  Reload
                </button>
                <button
                  type="button"
                  onClick={() => {
                    void save();
                  }}
                  disabled={!parsed.ok || saving}
                  className={primaryButtonCls}
                >
                  {saving ? "Saving…" : "Save"}
                </button>
              </div>
            </>
          )}
        </Card>
      </div>
    </div>
  );
}
