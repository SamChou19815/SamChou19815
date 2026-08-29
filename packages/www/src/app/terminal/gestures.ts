/**
 * Touch, translated into the mouse reports a terminal understands.
 *
 * Neither half of a gesture survives the trip to the backend otherwise.
 *
 * Scrolling: a phone has no wheel, and there is nothing for the browser's own
 * scrolling to move either — the app owns the alternate screen, so every row on
 * it comes from the backend, which scrolls only when a wheel tells it to. A drag
 * is metered out as wheel notches, one per `wheelRows` rows — the distance the
 * backend moves for one, which is what keeps the content under the finger.
 *
 * Tapping: xterm only reports a click when the browser synthesizes the mouse
 * events for one, and a phone does that late, grudgingly, and not at all if the
 * finger drifted — which is what made taps need a second go. So the tap is
 * reported straight from the gesture instead, and the whole gesture is claimed
 * at touchstart so no synthesized click doubles it.
 */

import type { Screen } from "./screen";

/** How far a finger may drift, in px, and still have meant a tap. */
const TAP_SLOP = 12;
/** And for how long, in ms — past this it was a press, not a tap. */
const TAP_TIME = 700;

type Gesture = { x: number; y: number; dragY: number; at: number; tap: boolean };

export type GestureOptions = {
  /** Rows one wheel notch scrolls, as the backend reports it. */
  wheelRows: number;
  /** Hands raw input bytes to the backend. */
  send: (bytes: string) => void;
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
    // The gesture is the app's from here: no synthesized mouse events for it,
    // no rubber-banding the page behind it, and no pull-to-refresh from a drag
    // that starts at the top. Focus comes with it, since the click that used to
    // carry it is one of the events just suppressed.
    event.preventDefault();
    screen.terminal.focus();
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
    const notchHeight = metrics.height * options.wheelRows;
    event.preventDefault();
    const notches = Math.trunc((gesture.dragY - y) / notchHeight);
    if (notches === 0) {
      return;
    }
    // Keep the remainder, so a slow drag still accumulates into a notch.
    gesture.dragY -= notches * notchHeight;
    // SGR wheel reports, the same ones xterm sends for a real wheel: button 64
    // is a notch up, 65 down. A wheel carries a position too, which the app
    // ignores, so the top left cell stands in for the finger.
    const report = notches > 0 ? "\x1b[<65;1;1M" : "\x1b[<64;1;1M";
    options.send(report.repeat(Math.abs(notches)));
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
    // An SGR press and release of the left button, the pair xterm sends for a
    // real click. The app acts on the press; the release keeps the backend's
    // button state honest.
    const { col, row } = cell;
    options.send(`\x1b[<0;${col};${row}M\x1b[<0;${col};${row}m`);
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
