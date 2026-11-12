import { useEffect, useRef, useState, type PointerEvent, type RefObject } from "react";

/** Distance from the edge of the scrolling page where dragging starts to scroll it, and its speed. */
const SCROLL_ZONE = 72;
const SCROLL_STEP = 14;

export type Drag = { from: number; to: number };

/** The nearest ancestor that scrolls, which is the page the dragged items are on. */
function scroller(el: HTMLElement | null): HTMLElement | null {
  for (let p = el?.parentElement; p; p = p.parentElement) {
    const overflow = getComputedStyle(p).overflowY;
    if ((overflow === "auto" || overflow === "scroll") && p.scrollHeight > p.clientHeight) return p;
  }
  return null;
}

/**
 * Dragging an item to another place with a pointer drag and not the HTML one, which a touch
 * screen does not start. `start(index)` goes on the handle of an item and the hook follows the
 * pointer from there, scrolling the page near its edges. `indexAt` tells which index the pointer
 * is over, so rows and a grid of cards can each say it their own way. The hook does not reorder
 * anything, `onMove` gets the old and the new index and the caller changes its list in answer.
 */
export function useDragReorder(
  body: RefObject<HTMLElement | null>,
  onMove: ((from: number, to: number) => void) | undefined,
  indexAt: (x: number, y: number) => number,
) {
  const [drag, setDrag] = useState<Drag | null>(null);
  // the latest pointer position, read by the scroll loop which runs between pointer events
  const pointer = useRef({ x: 0, y: 0 });
  // a mirror of the state for the window handlers, which outlive the render they were made in
  const current = useRef<Drag | null>(null);
  const moveRef = useRef(onMove);
  moveRef.current = onMove;
  const indexRef = useRef(indexAt);
  indexRef.current = indexAt;

  const set = (d: Drag | null) => {
    current.current = d;
    setDrag(d);
  };
  const follow = () => {
    const d = current.current;
    const to = d && indexRef.current(pointer.current.x, pointer.current.y);
    if (d && to !== d.to) set({ from: d.from, to: to! });
  };

  const start = (from: number) => (e: PointerEvent) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    pointer.current = { x: e.clientX, y: e.clientY };
    set({ from, to: from });
  };
  const finish = (drop: boolean) => {
    const d = current.current;
    set(null);
    if (d && drop && d.from !== d.to) moveRef.current?.(d.from, d.to);
  };

  // the handle holds the pointer capture, so its events reach the window while the drag lasts
  const dragging = drag !== null;
  useEffect(() => {
    if (!dragging) return;
    const moved = (e: globalThis.PointerEvent) => {
      pointer.current = { x: e.clientX, y: e.clientY };
      follow();
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    const page = scroller(body.current);
    let frame = 0;
    const scroll = () => {
      if (page) {
        const { top, bottom } = page.getBoundingClientRect();
        const y = pointer.current.y;
        if (y < top + SCROLL_ZONE) page.scrollTop -= SCROLL_STEP;
        else if (y > bottom - SCROLL_ZONE) page.scrollTop += SCROLL_STEP;
        // the items moved under the pointer, so the target changes without a pointer event
        follow();
      }
      frame = requestAnimationFrame(scroll);
    };
    frame = requestAnimationFrame(scroll);
    return () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      cancelAnimationFrame(frame);
    };
    // the handlers only read refs, so one set of them serves the whole drag
  }, [dragging]);

  return { drag, start };
}
