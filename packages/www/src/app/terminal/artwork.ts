/**
 * The card artwork laid over the terminal.
 *
 * The backend already draws it in-band as truecolor half-blocks — that is what
 * the native `dev-sam` binary shows — and reports where each one landed. Here
 * we cover those cells with the real asset, so the web gets full resolution and
 * the half-blocks become the fallback if an image fails to load.
 *
 * This is the one thing that cannot live in the backend: placing an `<img>`
 * needs the cell grid in pixels, which only the browser knows.
 */

import type { CellMetrics } from "./screen";

export type Artwork = {
  /** Places this frame's artwork and retires whatever it did not place. */
  sync(regions: string[], metrics: CellMetrics | null): void;
  dispose(): void;
};

export function mountArtwork(container: HTMLDivElement): Artwork {
  const layer = document.createElement("div");
  Object.assign(layer.style, {
    position: "absolute",
    inset: "0",
    pointerEvents: "none",
    overflow: "hidden",
  });
  container.appendChild(layer);

  // Pooled per URL: an image that stays on screen across frames is moved
  // rather than torn down and refetched, since rebuilding the layer every
  // frame would flicker at scroll rate. A pool rather than one element per
  // URL because an asset can appear more than once at the same time —
  // `flow.webp` and `graduation-sam.webp` each illustrate two timeline
  // events, and both can be on screen together.
  const pool = new Map<string, HTMLImageElement[]>();

  const claim = (url: string, index: number): HTMLImageElement => {
    let elements = pool.get(url);
    if (elements == null) {
      elements = [];
      pool.set(url, elements);
    }
    let element = elements[index];
    if (element == null) {
      element = document.createElement("img");
      element.src = url;
      element.alt = "";
      Object.assign(element.style, { position: "absolute", objectFit: "cover" });
      elements.push(element);
      layer.appendChild(element);
    }
    return element;
  };

  /** Drops every pooled element this frame did not claim. */
  const retire = (used: Map<string, number>): void => {
    for (const [url, elements] of pool) {
      for (const element of elements.splice(used.get(url) ?? 0)) {
        element.remove();
      }
      if (elements.length === 0) {
        pool.delete(url);
      }
    }
  };

  return {
    sync(regions, metrics) {
      if (regions.length === 0 && pool.size === 0) {
        return;
      }
      if (metrics == null) {
        return;
      }
      const used = new Map<string, number>();
      for (const region of regions) {
        // "x y cols rows top right bottom left url" in canvas cells; the app
        // owns the alternate screen, so cell (0, 0) is the viewport's top left.
        const [x, y, cols, rows, top, right, bottom, left, ...rest] = region.split(" ");
        const url = rest.join(" ");
        if (url === "") {
          continue;
        }
        const index = used.get(url) ?? 0;
        used.set(url, index + 1);
        // Placed at the full rectangle so the picture keeps its shape, then
        // cropped by what the pane's clipping took off each side — otherwise a
        // card half off the bottom would spill its artwork over the status bar.
        // The rectangle can start above the viewport, at a negative y, when a
        // post is scrolled part-way through one of its images.
        const inset = [
          Number(top) * metrics.height,
          Number(right) * metrics.width,
          Number(bottom) * metrics.height,
          Number(left) * metrics.width,
        ];
        Object.assign(claim(url, index).style, {
          left: `${metrics.offsetLeft + Number(x) * metrics.width}px`,
          top: `${metrics.offsetTop + Number(y) * metrics.height}px`,
          width: `${Number(cols) * metrics.width}px`,
          height: `${Number(rows) * metrics.height}px`,
          clipPath: inset.every((side) => side === 0)
            ? "none"
            : `inset(${inset.map((side) => `${side}px`).join(" ")})`,
        });
      }
      // A card that scrolled off, a duplicate that is no longer doubled up, or
      // the whole set at once when the app exits and reports no artwork at all.
      retire(used);
    },
    dispose() {
      layer.remove();
      pool.clear();
    },
  };
}
