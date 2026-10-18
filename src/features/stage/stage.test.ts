import { afterEach, describe, expect, it } from 'vitest';
import type { Activity } from '../agentui/activity';
import { animatorStats } from '../characters/animator';
import { autoLayout, cellAt, cellCenter, rowsOf, unitsFor, withPlaced } from './layout';
import { FLIGHT_MS, PULSE_MS, StageMotion } from './motion';
import { clearCells, placeCell, placedCells, restoreStage, snapshotStage } from './positions';
import {
  bubbleView,
  distanceToSegment,
  FADE_MS,
  needsFrames,
  packetPoint,
  shortText,
} from './scene';

const cluster = (id: string, workers: number) => ({
  id,
  name: id,
  color: '#fff',
  manager: `${id}-m`,
  workers: Array.from({ length: workers }, (_, i) => `${id}-w${i}`),
});

describe('stage auto layout', () => {
  it('puts the manager above its workers, centered, with the label on top', () => {
    const { cells, labels } = autoLayout([cluster('a', 3)], [], 20);
    expect(cells.get('a-m')).toEqual({ x: 1, y: 1 });
    expect(['a-w0', 'a-w1', 'a-w2'].map((w) => cells.get(w))).toEqual([
      { x: 0, y: 3 },
      { x: 1, y: 3 },
      { x: 2, y: 3 },
    ]);
    expect(labels[0]).toMatchObject({ id: 'a', manager: 'a-m', cell: { x: 1, y: 0 } });
  });

  it('flows 1–4 workstreams left to right and wraps; unlinked agents stand after them', () => {
    for (const count of [1, 2, 3, 4]) {
      const clusters = Array.from({ length: count }, (_, i) => cluster(`c${i}`, 2));
      const { cells } = autoLayout(clusters, ['free-1', 'free-2'], 7);
      const managers = clusters.map((c) => cells.get(c.manager));
      // Two clusters (2 wide + 1 gap) fit in 7 columns; the third wraps to the next band.
      expect(managers[0]).toEqual({ x: 0.5, y: 1 });
      if (count > 1) expect(managers[1]).toEqual({ x: 3.5, y: 1 });
      if (count > 2) expect(managers[2]).toEqual({ x: 0.5, y: 6 });
      if (count > 3) expect(managers[3]).toEqual({ x: 3.5, y: 6 });
      const lastBand = count > 2 ? 10 : 5;
      expect(cells.get('free-1')).toEqual({ x: 0, y: lastBand + 1 });
      expect(cells.get('free-2')).toEqual({ x: 1, y: lastBand + 1 });
      const all = [...cells.values()].map((c) => `${c.x},${c.y}`);
      expect(new Set(all).size, 'no two characters share a cell').toBe(all.length);
    }
    expect(autoLayout([], ['solo'], 5).cells.get('solo')).toEqual({ x: 0, y: 1 });
  });

  it('dragged positions win and the label follows its manager', () => {
    const auto = autoLayout([cluster('a', 1)], ['free'], 10);
    const placed = withPlaced(auto, { 'a-m': { x: 6, y: 2 }, gone: { x: 1, y: 1 } });
    expect(placed.cells.get('a-m')).toEqual({ x: 6, y: 2 });
    expect(placed.cells.has('gone')).toBe(false);
    expect(placed.labels[0]?.cell).toEqual({ x: 6, y: 1 });
  });

  it('maps cells to pixels and back, and shrinks rows to fit a short Stage', () => {
    const units = unitsFor(100, 60, 300, 4.6);
    expect(units).toEqual({ x: 100, y: 65.21739130434783 });
    expect(unitsFor(100, 80, 100, 4.6).y).toBe(80);
    const origin = { x: 20, y: 40 };
    const center = cellCenter({ x: 2, y: 3 }, { x: 100, y: 100 }, origin);
    expect(center).toEqual({ x: 270, y: 390 });
    expect(cellAt(center.x + 20, center.y - 10, { x: 100, y: 100 }, origin)).toEqual({
      x: 2,
      y: 3,
    });
    expect(rowsOf(autoLayout([cluster('a', 2)], [], 10))).toBeCloseTo(4.6);
  });
});

describe('stage positions', () => {
  afterEach(() => restoreStage({}));

  it('persist per workspace through the layout; auto arrange clears them', () => {
    placeCell('/w', 'chat-1', { x: 3, y: 2 });
    placeCell('/other', 'chat-1', { x: 9, y: 9 });
    const saved = JSON.parse(JSON.stringify(snapshotStage())) as Record<string, unknown>;
    restoreStage({});
    expect(placedCells('/w')).toEqual({});
    restoreStage(saved);
    expect(placedCells('/w')).toEqual({ 'chat-1': { x: 3, y: 2 } });
    clearCells('/w');
    expect(placedCells('/w')).toEqual({});
    expect(placedCells('/other')).toEqual({ 'chat-1': { x: 9, y: 9 } });
    restoreStage({ positions: { '/w': { a: { x: 'no', y: 1 }, b: { x: 1.4, y: 2.6 } } } });
    expect(placedCells('/w')).toEqual({ b: { x: 1, y: 3 } });
  });
});

describe('stage scene', () => {
  it('packets fly on an arc from the sender to the receiver', () => {
    const from = { x: 0, y: 100 };
    const to = { x: 200, y: 100 };
    expect(packetPoint(from, to, 0)).toEqual(from);
    expect(packetPoint(from, to, 1)).toEqual(to);
    const mid = packetPoint(from, to, 0.5);
    expect(mid.x).toBe(100);
    expect(mid.y).toBeLessThan(100);
    expect(distanceToSegment({ x: 50, y: 5 }, { x: 0, y: 0 }, { x: 100, y: 0 })).toBe(5);
  });

  it('bubbles show ~40 characters with an ellipsis and fade after done', () => {
    expect(shortText('Running npm test')).toBe('Running npm test');
    const long = 'Editing src/features/some/really/long/path/to/a/file.ts';
    expect(shortText(long)).toHaveLength(40);
    expect(shortText(long).endsWith('…')).toBe(true);
    const activity = (kind: Activity['kind'], at = 0): Activity => ({
      kind,
      text: kind === 'idle' ? '' : long,
      at,
      approximate: false,
    });
    expect(bubbleView(activity('tool'), 10 ** 9)).toMatchObject({ visible: true, full: long });
    expect(bubbleView(activity('idle'), 0).visible).toBe(false);
    expect(bubbleView(activity('done', 1000), 1000 + FADE_MS - 1)).toMatchObject({
      visible: true,
      fadesAt: 1000 + FADE_MS,
    });
    expect(bubbleView(activity('done', 1000), 1000 + FADE_MS).visible).toBe(false);
    expect(bubbleView(null, 0).visible).toBe(false);
  });

  it('nothing animates when idle; reduced motion never needs frames', () => {
    const still = {
      animate: true,
      animatedCharacters: 0,
      packets: 0,
      pulses: 0,
      pending: 0,
      glows: 0,
    };
    expect(needsFrames(still)).toBe(false);
    expect(needsFrames({ ...still, packets: 1 })).toBe(true);
    expect(needsFrames({ ...still, glows: 1 })).toBe(true);
    expect(needsFrames({ ...still, animatedCharacters: 3, animate: false })).toBe(false);
  });

  it('a handoff pulses the line, lands the packet and then the loop stops', () => {
    const queue: FrameRequestCallback[] = [];
    globalThis.requestAnimationFrame = (callback) => queue.push(callback);
    globalThis.cancelAnimationFrame = () => undefined;
    const motion = new StageMotion();
    motion.handoff('chat-1', 'terminal-2', '#fff', 0, true);
    expect(motion.heat('terminal-2', 'chat-1', 0)).toBe(1);
    expect(motion.aura('terminal-2', 10)).toBe('#fff');
    let now = 0;
    motion.run((t) => {
      motion.advance(t);
      return motion.busy > 0;
    });
    expect(animatorStats().running).toBe(1);
    while (queue.length > 0 && now < PULSE_MS * 3) {
      now += 16;
      queue.shift()?.(now);
    }
    expect(motion.receiving.get('terminal-2')).toBeGreaterThanOrEqual(FLIGHT_MS);
    expect(motion.busy).toBe(0);
    expect(motion.running).toBe(false);
    expect(animatorStats()).toMatchObject({ running: 0, scheduled: false });
    // Reduced motion: no flight, the receiver gets the packet at once.
    motion.handoff('chat-1', 'terminal-2', '#fff', 5000, false);
    expect(motion.packets(() => ({ x: 0, y: 0 }), 5000)).toEqual([]);
    expect(motion.receiving.get('terminal-2')).toBe(5000);
  });
});
