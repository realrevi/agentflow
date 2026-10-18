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
import { characterById, characters, loadCharacters, reducedMotion } from '../characters';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { themeVars } from '../theme/themeStore';
import { onHandoffSent, workstreamOf, workstreams } from '../workstreams/store';
import type { StageHooks } from './hooks';
import { createFrames } from './frame';
import { autoLayout, cellAt, cellCenter, rowsOf, unitsFor, withPlaced } from './layout';
import { createStagePointer, type Drag, type Hover } from './pointer';
import { PULSE_MS, StageMotion } from './motion';
import { clearCells, placeCell, placedCells } from './positions';
import { bubbleView, type Point } from './scene';
import { StageHistory } from './StageHistory';
import { StageOverlay } from './StageOverlay';
import { StageRenderer } from './StageRenderer';
import { Button } from '../../shared/ui/Button';

const PAD = 24;
const TOP = 40;

export default function StagePanel(props: { api: DockviewPanelApi; hooks: StageHooks }) {
  let host: HTMLDivElement | undefined;
  let canvas: HTMLCanvasElement | undefined;
  let renderer: StageRenderer | undefined;
  const motion = new StageMotion();
  const [size, setSize] = createSignal({ w: 0, h: 0 });
  const [visible, setVisible] = createSignal(props.api.isVisible);
  const [focused, setFocused] = createSignal(document.hasFocus());
  const [left, setLeft] = createSignal(props.hooks.isLeft());
  const [now, setNow] = createSignal(Date.now());
  const [tick, setTick] = createSignal(0);
  const [hover, setHover] = createSignal<Hover | null>(null);
  const [drag, setDrag] = createSignal<Drag | null>(null);
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

  const frames = createFrames({
    agents,
    pointOf,
    charSize,
    scale,
    speed: () => settings.characters.speed,
    animate,
    colors,
    motion,
  });

  /** Draws one frame; true while something still moves. */
  function draw(at: number): boolean {
    if (!renderer || !visible()) return false;
    motion.advance(at);
    const current = frames.frame(at);
    renderer.draw(current);
    return focused() && frames.moving(current, at);
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

  const pointer = createStagePointer({
    canvas: () => canvas,
    agents: () => agents().map((a) => a.id),
    charSize,
    pointOf,
    nameOf,
    drag,
    setDrag,
    setHover,
    setHistory,
    place: (panel, at) =>
      placeCell(settings.currentWorkspace, panel, cellAt(at.x, at.y, units(), origin)),
    hooks: props.hooks,
  });

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
        <Button size="sm" onClick={() => props.hooks.newWorkstream()}>
          {t('workstream.new')}
        </Button>
        <Button variant="secondary" size="sm" onClick={() => clearCells(settings.currentWorkspace)}>
          {t('stage.autoArrange')}
        </Button>
        <div class="af-stage-spacer" />
        <Button
          variant="secondary"
          size="sm"
          onClick={() => {
            props.hooks.move(left() ? 'center' : 'left');
            setLeft(props.hooks.isLeft());
          }}
        >
          {left() ? t('stage.moveCenter') : t('stage.moveLeft')}
        </Button>
      </div>
      <div ref={host} class="af-stage-body" onPointerLeave={() => setHover(null)}>
        <canvas
          ref={canvas}
          class="af-stage-canvas"
          role="img"
          aria-label={t('panel.stage.title')}
          onPointerDown={pointer.down}
          onPointerMove={pointer.move}
          onPointerUp={pointer.up}
          onDblClick={pointer.doubleClick}
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
