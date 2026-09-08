"use client";

import { useEffect, useRef } from "react";
import init, { start } from "../app/generated/sam_web";

/**
 * Phones and tablets, where the keyboard is an overlay that eats half the
 * viewport. A laptop with a touchscreen is not one of these: it has a real
 * keyboard, so the session there takes the screen over instead of drawing a
 * scrollable page.
 */
function isTouchOnlyDevice(): boolean {
  return window.matchMedia("(hover: none) and (pointer: coarse)").matches;
}

/**
 * The site itself is the wasm bundle: a Leptos app that renders the terminal,
 * the app, and every view the URL can name. React only provides the mount
 * point and loads the bundle.
 */
export default function SamSite(): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (container == null) {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        await init();
      } catch (error) {
        // Same surface the terminal had: the failure belongs on the page.
        container.textContent = `failed to load sam_web.wasm: ${String(error)}`;
        return;
      }
      if (!disposed) {
        start(container, isTouchOnlyDevice());
      }
    })();
    return () => {
      disposed = true;
    };
  }, []);

  return <div ref={containerRef} className="fixed inset-0" role="application" />;
}
