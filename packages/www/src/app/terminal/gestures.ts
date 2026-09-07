/**
 * Touch, told apart into the two things a finger can mean.
 *
 * Neither half of a gesture survives on its own. xterm's viewport is a
 * synthetic scrollbar over a canvas rather than a scrolling box, so a phone has
 * nothing to drag; and xterm only reports a click when the browser synthesizes
 * the mouse events for one, which a phone does late, grudgingly, and not at all
 * if the finger drifted — which is what made taps need a second go. So the whole
 * gesture is claimed at touchstart, and both halves are reported from here.
 *
 * Dragging is metered into steps of [`GestureOptions.drag`]`.rows` and handed
 * over a batch at a time, which is what keeps the content under the finger.
 * How far a step is depends on what is being moved: the full-screen app owns
 * the alternate screen and every row on it comes from the backend, which
 * scrolls only when a wheel tells it to, so a step there is one wheel notch.
 * The touch build has drawn its whole view into the terminal's own scrollback,
 * where moving is the emulator's business and not the backend's — a step is one
 * row, and no drag ever reaches the backend at all.
 */

import type { Screen } from "./screen";

/** How far a finger may drift, in px, and still have meant a tap. */
const TAP_SLOP = 12;
/** And for how long, in ms — past this it was a press, not a tap. */
const TAP_TIME = 700;

type Gesture = { x: number; y: number; dragY: number; at: number; tap: boolean };

export type GestureOptions = {
  /** What a drag moves, and the least it can move at a time. */
  drag: {
    /** Rows one step covers. */
    rows: number;
    /** Moves the content by `steps` steps; positive is further down. */
    scroll: (steps: number) => void;
  };
  /** Where a tap landed, in 1-based viewport cells. */
  onTap: (col: number, row: number) => void;
  /**
   * Whether claiming a gesture should take focus too. The click that would
   * otherwise have carried it is one of the events claiming suppresses, so a
   * terminal with a keyboard behind it has to take focus here — and one on a
   * phone, where focusing summons nothing, has no reason to.
   */
  focus: boolean;
};

/** Binds touch handling to the container; returns the unbind. */
export function bindGestures(
  container: HTMLDivElement,
  screen: Screen,
  options: GestureOptions,
): () => void {
  let gesture: Gesture | null = null;

  const onStart = (event: TouchEvent): void => {
    const touch = event.touches.length === 1 ? event.touches[0] : undefined;
    // Two fingers is a pinch; that one is the browser's to handle.
    if (touch == null) {
      gesture = null;
      return;
    }
    gesture = {
      x: touch.clientX,
      y: touch.clientY,
      dragY: touch.clientY,
      at: event.timeStamp,
      tap: true,
    };
    // The gesture is ours from here: no synthesized mouse events for it, no
    // rubber-banding the page behind it, and no pull-to-refresh from a drag
    // that starts at the top.
    event.preventDefault();
    if (options.focus) {
      screen.terminal.focus();
    }
  };

  const onMove = (event: TouchEvent): void => {
    const x = event.touches[0]?.clientX;
    const y = event.touches[0]?.clientY;
    if (gesture == null || x == null || y == null) {
      return;
    }
    if (Math.abs(x - gesture.x) > TAP_SLOP || Math.abs(y - gesture.y) > TAP_SLOP) {
      gesture.tap = false;
    }
    const metrics = screen.metrics();
    if (metrics == null) {
      return;
    }
    const stepHeight = metrics.height * options.drag.rows;
    event.preventDefault();
    const steps = Math.trunc((gesture.dragY - y) / stepHeight);
    if (steps === 0) {
      return;
    }
    // Keep the remainder, so a slow drag still accumulates into a step.
    gesture.dragY -= steps * stepHeight;
    options.drag.scroll(steps);
  };

  const onEnd = (event: TouchEvent): void => {
    const finished = gesture;
    gesture = null;
    if (finished == null || !finished.tap || event.timeStamp - finished.at > TAP_TIME) {
      return;
    }
    // Where the finger landed, not where it left: within the slop they are the
    // same cell, and the landing is what the visitor aimed at.
    const cell = screen.cellAt(finished.x, finished.y);
    if (cell == null) {
      return;
    }
    options.onTap(cell.col, cell.row);
  };

  const onCancel = (): void => {
    gesture = null;
  };

  container.addEventListener("touchstart", onStart, { passive: false });
  container.addEventListener("touchmove", onMove, { passive: false });
  container.addEventListener("touchend", onEnd);
  container.addEventListener("touchcancel", onCancel);

  return () => {
    container.removeEventListener("touchstart", onStart);
    container.removeEventListener("touchmove", onMove);
    container.removeEventListener("touchend", onEnd);
    container.removeEventListener("touchcancel", onCancel);
  };
}
