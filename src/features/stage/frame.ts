// What one Stage frame shows: the characters (sprite state and since when, glow, aura),
// the manager-worker lines with their heat, flying and bouncing packets; and whether any of
// it still moves (the loop stops when nothing does).
import { agentState, glowing } from '../agentui';
import { characterById, spriteFor } from '../characters';
import { Glide } from '../characters/motion';
import { frameAt, playbackOf } from '../characters/playback';
import type { CharacterRef, SpriteState } from '../characters/types';
import { workstreams } from '../workstreams/store';
import { StageMotion } from './motion';
import { needsFrames, type Point } from './scene';
import type { CharDraw, FrameInput, LineDraw } from './StageRenderer';

const RECEIVE_MS = 700;

export interface FrameDeps {
  agents: () => readonly { id: string; character: CharacterRef | null }[];
  pointOf: (panel: string) => Point | undefined;
  charSize: () => number;
  scale: () => number;
  speed: () => number;
  animate: () => boolean;
  colors: () => FrameInput['colors'];
  motion: StageMotion;
}

export function createFrames(deps: FrameDeps) {
  const { motion } = deps;
  const since = new Map<string, { state: SpriteState; at: number }>();
  // Characters glide to their spots (drag, auto-arrange, new layout) instead of jumping.
  const glide = new Glide();
  const shown = new Map<string, Point>();
  const shownPoint = (panel: string) => shown.get(panel) ?? deps.pointOf(panel);

  const spriteState = (panel: string, at: number): SpriteState => {
    const received = motion.receiving.get(panel);
    if (received !== undefined && at - received < RECEIVE_MS) return 'receive';
    if (received !== undefined) motion.receiving.delete(panel);
    return spriteFor(agentState(panel));
  };

  const animating = (char: CharDraw, at: number) => {
    const playback = playbackOf(char.state, deps.animate());
    if (playback.kind === 'still') return false;
    const frames = char.sheet.states[char.state].length;
    return frameAt(playback, frames, at - char.since, char.sheet.frameMs) !== null;
  };

  function frame(at: number): FrameInput {
    const chars: CharDraw[] = [];
    shown.clear();
    for (const info of deps.agents()) {
      const sheet = info.character ? characterById(info.character.id) : undefined;
      const target = deps.pointOf(info.id);
      if (!info.character || !sheet || !target) {
        glide.forget(info.id);
        continue;
      }
      const point = glide.at(info.id, target, at, deps.animate());
      shown.set(info.id, point);
      const state = spriteState(info.id, at);
      const known = since.get(info.id);
      const started = known?.state === state ? known.at : at;
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
    const half = deps.charSize() / 2;
    const lines: LineDraw[] = [];
    const packets = motion.packets(shownPoint, at);
    for (const ws of workstreams()) {
      const top = shownPoint(ws.manager);
      for (const worker of ws.workers) {
        const bottom = shownPoint(worker);
        if (!top || !bottom) continue;
        lines.push({
          from: { x: top.x, y: top.y + half },
          to: { x: bottom.x, y: bottom.y - half },
          color: ws.color,
          heat: motion.heat(ws.manager, worker, at),
        });
      }
      const sender = ws.pending ? shownPoint(ws.pending.from) : undefined;
      if (sender) {
        const lift = deps.charSize() * 0.8;
        packets.push({ at: StageMotion.bounce(sender, lift, at, deps.animate()), color: ws.color });
      }
    }
    return {
      scale: deps.scale(),
      speed: deps.speed(),
      animate: deps.animate(),
      now: at,
      chars,
      lines,
      packets,
      colors: deps.colors(),
    };
  }

  /** Whether `current` still moves at `at` (also needs the window focused, checked outside). */
  function moving(current: FrameInput, at: number): boolean {
    const pending = workstreams().filter((ws) => ws.pending).length;
    return needsFrames({
      animate: deps.animate(),
      animatedCharacters: current.chars.filter((c) => animating(c, at)).length + glide.gliding,
      packets: current.packets.length - pending,
      pulses: motion.busy,
      pending,
      glows: current.chars.filter((c) => c.glow).length,
    });
  }

  return { frame, moving };
}
