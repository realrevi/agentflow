// Lazy-loaded dock panel: the Agent Stage (default export). Every open agent panel as its
// pixel character on one canvas, clustered by workstream (manager on top, workers below),
// with lines in the workstream color, packets for handoffs and DOM bubbles. It draws on
// change; the shared loop runs only while something moves, never while hidden or while
// the window is in the background.
import './stage.css';
import type { DockviewPanelApi } from 'dockview-core';
import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  Show,
  untrack,
} from 'solid-js';
import { activityOf, agentState, glowing } from '../agentui';
import { createPromptHistory, HistoryPopover } from '../agentui/components';
import { characterById, characters, loadCharacters, reducedMotion, spriteFor } from '../characters';
import { frameAt, playbackOf } from '../characters/playback';
import type { SpriteState } from '../characters/types';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { themeVars } from '../theme/themeStore';
import { onHandoffSent, workstreamOf, workstreams } from '../workstreams/store';
import type { StageHooks } from './hooks';
import { autoLayout, cellAt, cellCenter, rowsOf, unitsFor, withPlaced } from './layout';
import { PULSE_MS, StageMotion } from './motion';
import { clearCells, placeCell, placedCells } from './positions';
import { bubbleView, distanceToSegment, needsFrames, type Point } from './scene';
import { StageOverlay } from './StageOverlay';
import { StageRenderer, type CharDraw, type FrameInput, type LineDraw } from './StageRenderer';

const PAD = 24;
const TOP = 40;
const RECEIVE_MS = 700;
const DRAG_PX = 4;

export default function StagePanel(props: { api: DockviewPanelApi; hooks: StageHooks }) {
  let host: HTMLDivElement | undefined;
  let canvas: HTMLCanvasElement | undefined;
  let renderer: StageRenderer | undefined;
  const motion = new StageMotion();
  const since = new Map<string, { state: SpriteState; at: number }>();
  const [size, setSize] = createSignal({ w: 0, h: 0 });
  const [visible, setVisible] = createSignal(props.api.isVisible);
  const [focused, setFocused] = createSignal(document.hasFocus());
  const [left, setLeft] = createSignal(props.hooks.isLeft());
  const [now, setNow] = createSignal(Date.now());
  const [tick, setTick] = createSignal(0);
  const [hover, setHover] = createSignal<{ panel: string; title: string; at: Point } | null>(null);
  const [drag, setDrag] = createSignal<{
    panel: string;
    start: Point;
    at: Point;
    moved: boolean;
  } | null>(null);
  const [history, setHistory] = createSignal<{ panel: string; at: Point } | null>(null);

  const scale = () => settings.workstreams.stageScale;
  const charSize = () => 16 * scale();
  const unit = () => Math.round(charSize() * 1.9);
  const animate = () => settings.workstreams.stageAnimations && !reducedMotion();
  const origin = { x: PAD, y: TOP };

  const agents = createMemo(() => props.hooks.panels().filter((p) => p.agent && p.character));
  const placement = createMemo(() => {
    const ids = new Set(agents().map((a) => a.id));
    const clusters = workstreams()
      .filter((ws) => ids.has(ws.manager))
      .map((ws) => ({
        id: ws.id,
        name: ws.name,
        color: ws.color,
        manager: ws.manager,
        workers: ws.workers.filter((w) => ids.has(w)),
      }));
    const linked = new Set(clusters.flatMap((c) => [c.manager, ...c.workers]));
    const loose = agents()
      .map((a) => a.id)
      .filter((id) => !linked.has(id));
    const cols = Math.max(1, Math.floor((size().w - PAD * 2) / unit()));
    return withPlaced(autoLayout(clusters, loose, cols), placedCells(settings.currentWorkspace));
  });
  const units = createMemo(() =>
    unitsFor(unit(), charSize() * 1.3, size().h - TOP - PAD, rowsOf(placement())),
  );
  const pointOf = (panel: string): Point | undefined => {
    const dragged = drag();
    if (dragged?.panel === panel && dragged.moved) return dragged.at;
    const cell = placement().cells.get(panel);
    return cell ? cellCenter(cell, units(), origin) : undefined;
  };
  const nameOf = (panel: string): string => {
    const info = agents().find((a) => a.id === panel);
    const sheet = info?.character ? characterById(info.character.id) : undefined;
    return workstreamOf(panel)?.names[panel] ?? sheet?.name ?? info?.title ?? panel;
  };
  const colors = createMemo(() => {
    themeVars();
    const style = getComputedStyle(document.documentElement);
    const css = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
    const glow = settings.workstreams.glowColor ?? settings.appearance.agentBold.color;
    return { bg: css('--af-panel-bg', '#1e1e1e'), grid: css('--af-border', '#3a3a3a'), glow };
  });

  const spriteState = (panel: string, at: number): SpriteState => {
    const received = motion.receiving.get(panel);
    if (received !== undefined && at - received < RECEIVE_MS) return 'receive';
    if (received !== undefined) motion.receiving.delete(panel);
    return spriteFor(agentState(panel));
  };
  const animating = (char: CharDraw, at: number) => {
    const playback = playbackOf(char.state, animate());
    if (playback.kind === 'still') return false;
    const frames = char.sheet.states[char.state].length;
    return frameAt(playback, frames, at - char.since, char.sheet.frameMs) !== null;
  };

  function frame(at: number): FrameInput {
    const chars: CharDraw[] = [];
    for (const info of agents()) {
      const sheet = info.character ? characterById(info.character.id) : undefined;
      const point = pointOf(info.id);
      if (!info.character || !sheet || !point) continue;
      const state = spriteState(info.id, at);
      const started = since.get(info.id)?.state === state ? (since.get(info.id)?.at ?? at) : at;
      since.set(info.id, { state, at: started });
      chars.push({
        panel: info.id,
        sheet,
        palette: info.character.palette,
        state,
        since: started,
        at: point,
        glow: glowing(info.id),
        aura: motion.aura(info.id, at),
      });
    }
    const lines: LineDraw[] = [];
    for (const ws of workstreams()) {
      const top = pointOf(ws.manager);
      for (const worker of ws.workers) {
        const bottom = pointOf(worker);
        if (!top || !bottom) continue;
        lines.push({
          from: { x: top.x, y: top.y + charSize() / 2 },
          to: { x: bottom.x, y: bottom.y - charSize() / 2 },
          color: ws.color,
          heat: motion.heat(ws.manager, worker, at),
        });
      }
    }
    const packets = motion.packets(pointOf, at);
    for (const ws of workstreams()) {
      const sender = ws.pending ? pointOf(ws.pending.from) : undefined;
      if (sender)
        packets.push({
          at: StageMotion.bounce(sender, charSize() * 0.8, at, animate()),
          color: ws.color,
        });
    }
    return {
      scale: scale(),
      speed: settings.characters.speed,
      animate: animate(),
      now: at,
      chars,
      lines,
      packets,
      colors: colors(),
    };
  }

  /** Draws one frame; true while something still moves. */
  function draw(at: number): boolean {
    if (!renderer || !visible()) return false;
    motion.advance(at);
    const current = frame(at);
    renderer.draw(current);
    const pending = workstreams().filter((ws) => ws.pending).length;
    return (
      focused() &&
      needsFrames({
        animate: animate(),
        animatedCharacters: current.chars.filter((c) => animating(c, at)).length,
        packets: current.packets.length - pending,
        pulses: motion.busy,
        pending,
        glows: current.chars.filter((c) => c.glow).length,
      })
    );
  }

  // Redraw on any change; start the loop only when something moves.
  createEffect(() => {
    size();
    visible();
    focused();
    tick();
    placement();
    colors();
    characters();
    animate();
    drag();
    for (const info of agents()) {
      agentState(info.id);
      glowing(info.id);
    }
    for (const ws of workstreams()) void ws.pending?.id;
    untrack(() => {
      if (draw(performance.now())) motion.run(draw);
    });
  });

  // Bubbles fade a while after "done" or an error: one timer for the next fade.
  createEffect(() => {
    let next = Number.POSITIVE_INFINITY;
    for (const info of agents()) {
      const fades = bubbleView(activityOf(info.id), now()).fadesAt;
      if (fades !== null) next = Math.min(next, fades);
    }
    if (!Number.isFinite(next)) return;
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(0, next - Date.now()) + 30);
    onCleanup(() => window.clearTimeout(timer));
  });

  onCleanup(
    onHandoffSent((ws, handoff) => {
      motion.handoff(handoff.from, handoff.to, ws.color, performance.now(), animate());
      setTick((n) => n + 1);
      // Reduced motion: the line lights up without a pulse, then goes back.
      if (!animate()) window.setTimeout(() => setTick((n) => n + 1), PULSE_MS + 50);
    }),
  );

  const hit = (x: number, y: number): string | null => {
    const half = charSize() / 2 + 4;
    for (const info of agents()) {
      const at = pointOf(info.id);
      if (at && Math.abs(at.x - x) <= half && Math.abs(at.y - y) <= half) return info.id;
    }
    return null;
  };
  const hitLine = (x: number, y: number): { ws: string; pair: [string, string] } | null => {
    for (const ws of workstreams()) {
      const top = pointOf(ws.manager);
      for (const worker of ws.workers) {
        const bottom = pointOf(worker);
        if (top && bottom && distanceToSegment({ x, y }, top, bottom) < 6) {
          return { ws: ws.id, pair: [ws.manager, worker] };
        }
      }
    }
    return null;
  };
  const local = (event: PointerEvent | MouseEvent): Point => {
    const box = canvas?.getBoundingClientRect();
    return { x: event.clientX - (box?.left ?? 0), y: event.clientY - (box?.top ?? 0) };
  };

  const onPointerDown = (event: PointerEvent) => {
    const at = local(event);
    const panel = hit(at.x, at.y);
    if (!panel) return;
    setDrag({ panel, start: at, at, moved: false });
    try {
      canvas?.setPointerCapture(event.pointerId);
    } catch {
      // Not every input device can be captured; dragging still works inside the canvas.
    }
  };
  const onPointerMove = (event: PointerEvent) => {
    const at = local(event);
    const dragging = drag();
    if (dragging) {
      const moved =
        dragging.moved || Math.hypot(at.x - dragging.start.x, at.y - dragging.start.y) > DRAG_PX;
      setDrag({ ...dragging, at, moved });
      setHover(null);
      return;
    }
    const panel = hit(at.x, at.y);
    const point = panel ? pointOf(panel) : undefined;
    setHover(
      panel && point
        ? { panel, title: nameOf(panel), at: { x: point.x + charSize() / 2 + 8, y: point.y } }
        : null,
    );
    if (canvas) canvas.style.cursor = panel || hitLine(at.x, at.y) ? 'pointer' : 'default';
  };
  const onPointerUp = (event: PointerEvent) => {
    const dragging = drag();
    setDrag(null);
    if (dragging?.moved) {
      placeCell(
        settings.currentWorkspace,
        dragging.panel,
        cellAt(dragging.at.x, dragging.at.y, units(), origin),
      );
      return;
    }
    if (dragging) {
      props.hooks.focus(dragging.panel);
      return;
    }
    const at = local(event);
    const line = hitLine(at.x, at.y);
    if (line) props.hooks.openTimeline(line.ws, line.pair);
  };
  const onDoubleClick = (event: MouseEvent) => {
    const at = local(event);
    const panel = hit(at.x, at.y);
    const point = panel ? pointOf(panel) : undefined;
    if (panel && point) setHistory({ panel, at: point });
  };

  onMount(() => {
    void loadCharacters();
    if (canvas) renderer = new StageRenderer(canvas);
    const observer = new ResizeObserver(([entry]) => {
      if (!entry || !renderer) return;
      renderer.resize(entry.contentRect.width, entry.contentRect.height);
      setSize({ w: entry.contentRect.width, h: entry.contentRect.height });
    });
    if (host) observer.observe(host);
    const onFocus = () => setFocused(true);
    const onBlur = () => setFocused(false);
    window.addEventListener('focus', onFocus);
    window.addEventListener('blur', onBlur);
    const subscriptions = [
      props.api.onDidVisibilityChange((event) => setVisible(event.isVisible)),
      props.api.onDidGroupChange(() => setLeft(props.hooks.isLeft())),
      props.api.onDidLocationChange(() => setLeft(props.hooks.isLeft())),
    ];
    onCleanup(() => {
      observer.disconnect();
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('blur', onBlur);
      subscriptions.forEach((subscription) => subscription.dispose());
      motion.halt();
    });
  });

  const labels = () =>
    placement().labels.map((label) => {
      const at = cellCenter(label.cell, units(), origin);
      return { ...label, at: { x: at.x, y: at.y + units().y / 2 - 10 } };
    });

  return (
    <div class="af-stage">
      <div class="af-stage-bar">
        <button type="button" class="af-button small" onClick={() => props.hooks.newWorkstream()}>
          {t('workstream.new')}
        </button>
        <button
          type="button"
          class="af-button secondary small"
          onClick={() => clearCells(settings.currentWorkspace)}
        >
          {t('stage.autoArrange')}
        </button>
        <div class="af-stage-spacer" />
        <button
          type="button"
          class="af-button secondary small"
          onClick={() => {
            props.hooks.move(left() ? 'center' : 'left');
            setLeft(props.hooks.isLeft());
          }}
        >
          {left() ? t('stage.moveCenter') : t('stage.moveLeft')}
        </button>
      </div>
      <div ref={host} class="af-stage-body" onPointerLeave={() => setHover(null)}>
        <canvas
          ref={canvas}
          class="af-stage-canvas"
          role="img"
          aria-label={t('panel.stage.title')}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onDblClick={onDoubleClick}
        />
        <StageOverlay
          ids={agents().map((a) => a.id)}
          character={(id) => {
            const at = pointOf(id);
            return at ? { id, name: nameOf(id), at } : undefined;
          }}
          labels={labels()}
          size={charSize()}
          now={now()}
          hover={hover()}
        />
        <Show when={agents().length === 0}>
          <p class="af-stage-empty">{t('stage.empty')}</p>
        </Show>
        <Show when={history()} keyed>
          {(open) => (
            <StageHistory {...open} hooks={props.hooks} onClose={() => setHistory(null)} />
          )}
        </Show>
      </div>
    </div>
  );
}

function StageHistory(props: { panel: string; at: Point; hooks: StageHooks; onClose: () => void }) {
  let anchor: HTMLDivElement | undefined;
  const where = props.hooks.history(props.panel);
  const entries = where ? createPromptHistory(where.workspace, where.key).entries : () => [];
  return (
    <div
      ref={anchor}
      class="af-stage-history"
      style={{ left: `${props.at.x}px`, top: `${props.at.y}px` }}
    >
      <Show when={anchor}>
        {(element) => (
          <HistoryPopover
            entries={entries()}
            anchor={element()}
            onPick={() => props.hooks.focus(props.panel)}
            onClose={props.onClose}
          />
        )}
      </Show>
    </div>
  );
}
