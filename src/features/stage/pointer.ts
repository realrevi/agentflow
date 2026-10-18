// Pointer input on the Stage canvas: hover shows the tooltip, a click focuses the panel, a
// drag moves the character (saved per workspace), a click on a line opens the Timeline for
// that manager-worker pair, a double-click shows the panel's prompt history.
import { workstreams } from '../workstreams/store';
import type { StageHooks } from './hooks';
import { distanceToSegment, type Point } from './scene';

const DRAG_PX = 4;
const LINE_PX = 6;

export interface Drag {
  panel: string;
  start: Point;
  at: Point;
  moved: boolean;
}

export interface Hover {
  panel: string;
  title: string;
  at: Point;
}

export interface PointerDeps {
  canvas: () => HTMLCanvasElement | undefined;
  agents: () => readonly string[];
  charSize: () => number;
  pointOf: (panel: string) => Point | undefined;
  nameOf: (panel: string) => string;
  drag: () => Drag | null;
  setDrag: (drag: Drag | null) => void;
  setHover: (hover: Hover | null) => void;
  setHistory: (open: { panel: string; at: Point } | null) => void;
  /** Saves a dragged character's new place. */
  place: (panel: string, at: Point) => void;
  hooks: StageHooks;
}

export function createStagePointer(deps: PointerDeps) {
  const hit = (x: number, y: number): string | null => {
    const half = deps.charSize() / 2 + 4;
    for (const panel of deps.agents()) {
      const at = deps.pointOf(panel);
      if (at && Math.abs(at.x - x) <= half && Math.abs(at.y - y) <= half) return panel;
    }
    return null;
  };
  const hitLine = (x: number, y: number): { ws: string; pair: [string, string] } | null => {
    for (const ws of workstreams()) {
      const top = deps.pointOf(ws.manager);
      for (const worker of ws.workers) {
        const bottom = deps.pointOf(worker);
        if (top && bottom && distanceToSegment({ x, y }, top, bottom) < LINE_PX) {
          return { ws: ws.id, pair: [ws.manager, worker] };
        }
      }
    }
    return null;
  };
  const local = (event: PointerEvent | MouseEvent): Point => {
    const box = deps.canvas()?.getBoundingClientRect();
    return { x: event.clientX - (box?.left ?? 0), y: event.clientY - (box?.top ?? 0) };
  };

  return {
    down(event: PointerEvent) {
      const at = local(event);
      const panel = hit(at.x, at.y);
      if (!panel) return;
      deps.setDrag({ panel, start: at, at, moved: false });
      try {
        deps.canvas()?.setPointerCapture(event.pointerId);
      } catch {
        // Not every input device can be captured; dragging still works inside the canvas.
      }
    },
    move(event: PointerEvent) {
      const at = local(event);
      const dragging = deps.drag();
      if (dragging) {
        const far = Math.hypot(at.x - dragging.start.x, at.y - dragging.start.y) > DRAG_PX;
        deps.setDrag({ ...dragging, at, moved: dragging.moved || far });
        deps.setHover(null);
        return;
      }
      const panel = hit(at.x, at.y);
      const point = panel ? deps.pointOf(panel) : undefined;
      deps.setHover(
        panel && point
          ? {
              panel,
              title: deps.nameOf(panel),
              at: { x: point.x + deps.charSize() / 2 + 8, y: point.y },
            }
          : null,
      );
      const canvas = deps.canvas();
      if (canvas) canvas.style.cursor = panel || hitLine(at.x, at.y) ? 'pointer' : 'default';
    },
    /** A drag drops the character; anything else is a click: a character focuses its panel,
     * a line opens the Timeline. (The pointerdown may not reach the canvas: dockview activates
     * an inactive group on it, so clicks are judged on pointerup.) */
    up(event: PointerEvent) {
      const dragging = deps.drag();
      deps.setDrag(null);
      if (dragging?.moved) {
        deps.place(dragging.panel, dragging.at);
        return;
      }
      const at = local(event);
      const panel = hit(at.x, at.y);
      const line = panel ? null : hitLine(at.x, at.y);
      // After dockview handled the event (it re-activates the Stage's own tab otherwise).
      window.setTimeout(() => {
        if (panel) deps.hooks.focus(panel);
        else if (line) deps.hooks.openTimeline(line.ws, line.pair);
      }, 0);
    },
    doubleClick(event: MouseEvent) {
      const at = local(event);
      const panel = hit(at.x, at.y);
      const point = panel ? deps.pointOf(panel) : undefined;
      if (panel && point) deps.setHistory({ panel, at: point });
    },
  };
}
