/**
 * The card artwork laid over the terminal.
 *
 * The backend reserves a cell box for each picture, frames it, and reports
 * where it landed. Here we cover those cells with the real asset, so the frame
 * is only ever what shows while the file loads — or what stays if it 404s.
 *
 * This is the one thing that cannot live in the backend: placing an `<img>`
 * needs the cell grid in pixels, which only the browser knows.
 */

import type { CellMetrics } from "./screen";

export type Artwork = {
  /**
   * Places this frame's artwork and retires whatever it did not place.
   * `scrollRows` is how far the terminal has scrolled past the top of the
   * canvas — zero for an app that owns the screen, and what the touch build's
   * page has moved under the finger since it was drawn.
   */
  sync(regions: string[], metrics: CellMetrics | null, scrollRows: number): void;
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
      // The touch build reports every picture on the page at once, most of them
      // rows below the fold, so the ones that are not on screen wait until they
      // are scrolled to. An app that owns the screen only ever reports what is
      // already in view, where this changes nothing.
      element.loading = "lazy";
      // Anchored at the layer's origin and moved from there by a transform, so
      // that scrolling a page of artwork writes a property the browser can
      // answer without laying the document out again. `left`/`top` cannot be:
      // every step would dirty the geometry of every picture on the page.
      Object.assign(element.style, {
        position: "absolute",
        left: "0",
        top: "0",
        objectFit: "cover",
      });
      elements.push(element);
      layer.appendChild(element);
    }
    return element;
  };

  /** What was last written to each element, so an unchanged frame writes none. */
  const placed = new WeakMap<HTMLImageElement, string>();

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
    sync(regions, metrics, scrollRows) {
      if (regions.length === 0 && pool.size === 0) {
        return;
      }
      if (metrics == null) {
        return;
      }
      const used = new Map<string, number>();
      for (const region of regions) {
        // "x y cols rows top right bottom left url" in canvas cells. Cell
        // (0, 0) is the canvas's top left: the viewport's, for the app that
        // owns the alternate screen, and the first row of the touch build's
        // page, which the visitor has scrolled `scrollRows` past.
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
        const style = {
          transform: `translate(${metrics.offsetLeft + Number(x) * metrics.width}px, ${
            metrics.offsetTop + (Number(y) - scrollRows) * metrics.height
          }px)`,
          width: `${Number(cols) * metrics.width}px`,
          height: `${Number(rows) * metrics.height}px`,
          clipPath: inset.every((side) => side === 0)
            ? "none"
            : `inset(${inset.map((side) => `${side}px`).join(" ")})`,
        };
        const element = claim(url, index);
        // Only the transform changes while a page is scrolled; writing the size
        // and the crop again every row would be three more style invalidations
        // per picture for nothing.
        const key = `${style.transform}|${style.width}|${style.height}|${style.clipPath}`;
        if (placed.get(element) !== key) {
          placed.set(element, key);
          Object.assign(element.style, style);
        }
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
