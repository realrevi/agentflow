// Draws one Stage frame on its canvas: the themed pixel grid, manager-worker lines in the
// workstream color (brighter while a handoff runs), auras, the gold glow of characters that
// need the user, the characters (cached sprite strips, crisp pixels) and packets.
import { frameAt, playbackOf } from '../characters/playback';
import { deviceScale, strip } from '../characters/renderer';
import type { SpriteSheet, SpriteState } from '../characters/types';
import type { Point } from './scene';

export interface CharDraw {
  panel: string;
  sheet: SpriteSheet;
  palette: number;
  state: SpriteState;
  /** When the sprite state started (performance.now()). */
  since: number;
  at: Point;
  glow: boolean;
  /** Workstream color of a running handoff around it. */
  aura: string | null;
}

export interface LineDraw {
  from: Point;
  to: Point;
  color: string;
  /** 0 = resting, 1 = a handoff right now. */
  heat: number;
}

export interface PacketDraw {
  at: Point;
  color: string;
}

export interface FrameInput {
  scale: number;
  speed: number;
  animate: boolean;
  now: number;
  chars: readonly CharDraw[];
  lines: readonly LineDraw[];
  packets: readonly PacketDraw[];
  colors: { bg: string; grid: string; glow: string };
}

const GRID = 16;

export class StageRenderer {
  private readonly ctx: CanvasRenderingContext2D | null;
  private ratio = 1;
  private width = 0;
  private height = 0;
  /** The grid, drawn once per size and theme. */
  private backdrop: { key: string; canvas: HTMLCanvasElement } | null = null;

  constructor(private readonly canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext('2d');
  }

  resize(width: number, height: number): void {
    this.ratio = Math.max(1, Math.round(window.devicePixelRatio || 1));
    this.width = Math.max(1, Math.floor(width));
    this.height = Math.max(1, Math.floor(height));
    this.canvas.width = this.width * this.ratio;
    this.canvas.height = this.height * this.ratio;
  }

  draw(frame: FrameInput): void {
    const ctx = this.ctx;
    if (!ctx) return;
    ctx.setTransform(this.ratio, 0, 0, this.ratio, 0, 0);
    ctx.imageSmoothingEnabled = false;
    this.background(ctx, frame);
    for (const line of frame.lines) this.line(ctx, line, frame);
    const size = 16 * frame.scale;
    for (const char of frame.chars) {
      if (char.aura) this.halo(ctx, char.at, size * 0.85, char.aura, 0.35);
      if (char.glow) this.glow(ctx, char.at, size, frame);
    }
    for (const char of frame.chars) this.character(ctx, char, frame);
    for (const packet of frame.packets) this.packet(ctx, packet, frame.scale);
  }

  private background(ctx: CanvasRenderingContext2D, frame: FrameInput): void {
    const key = `${this.width}x${this.height}@${this.ratio}:${frame.colors.bg}:${frame.colors.grid}`;
    if (this.backdrop?.key !== key) {
      const canvas = document.createElement('canvas');
      canvas.width = this.canvas.width;
      canvas.height = this.canvas.height;
      const bg = canvas.getContext('2d');
      if (bg) {
        bg.setTransform(this.ratio, 0, 0, this.ratio, 0, 0);
        bg.fillStyle = frame.colors.bg;
        bg.fillRect(0, 0, this.width, this.height);
        bg.fillStyle = frame.colors.grid;
        for (let y = GRID; y < this.height; y += GRID) {
          for (let x = GRID; x < this.width; x += GRID) bg.fillRect(x, y, 1, 1);
        }
      }
      this.backdrop = { key, canvas };
    }
    ctx.drawImage(this.backdrop.canvas, 0, 0, this.width, this.height);
  }

  private line(ctx: CanvasRenderingContext2D, line: LineDraw, frame: FrameInput): void {
    const pulse = frame.animate && line.heat > 0 ? 0.5 + 0.5 * Math.sin(frame.now / 90) : 1;
    ctx.save();
    ctx.globalAlpha = 0.35 + 0.6 * line.heat * pulse;
    ctx.strokeStyle = line.color;
    ctx.lineWidth = line.heat > 0 ? 3 : 2;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(Math.round(line.from.x), Math.round(line.from.y));
    ctx.lineTo(Math.round(line.to.x), Math.round(line.to.y));
    ctx.stroke();
    ctx.restore();
  }

  private halo(
    ctx: CanvasRenderingContext2D,
    at: Point,
    radius: number,
    color: string,
    alpha: number,
  ) {
    const gradient = ctx.createRadialGradient(at.x, at.y, radius * 0.3, at.x, at.y, radius);
    gradient.addColorStop(0, color);
    gradient.addColorStop(1, 'transparent');
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.fillStyle = gradient;
    ctx.fillRect(at.x - radius, at.y - radius, radius * 2, radius * 2);
    ctx.restore();
  }

  /** Gold glow; it breathes unless motion is reduced (then it holds still). */
  private glow(ctx: CanvasRenderingContext2D, at: Point, size: number, frame: FrameInput): void {
    const breathe = frame.animate ? 0.45 + 0.25 * Math.sin(frame.now / 300) : 0.55;
    this.halo(ctx, at, size * 0.95, frame.colors.glow, breathe);
    ctx.save();
    ctx.strokeStyle = frame.colors.glow;
    ctx.lineWidth = 2;
    const half = size / 2 + 4;
    ctx.strokeRect(Math.round(at.x - half), Math.round(at.y - half), half * 2, half * 2);
    ctx.restore();
  }

  private character(ctx: CanvasRenderingContext2D, char: CharDraw, frame: FrameInput): void {
    const scale = deviceScale(frame.scale);
    const entry = strip(char.sheet, char.palette, scale, char.state);
    const playback = playbackOf(char.state, frame.animate);
    const frameMs = char.sheet.frameMs / Math.max(0.25, frame.speed);
    const index =
      frameAt(playback, entry.frames, frame.now - char.since, frameMs) ??
      (playback.kind === 'once' && playback.hold === 'last' ? entry.frames - 1 : 0);
    const size = char.sheet.size * frame.scale;
    ctx.drawImage(
      entry.source,
      index * entry.cell,
      0,
      entry.cell,
      entry.cell,
      Math.round(char.at.x - size / 2),
      Math.round(char.at.y - size / 2),
      size,
      size,
    );
  }

  private packet(ctx: CanvasRenderingContext2D, packet: PacketDraw, scale: number): void {
    const size = Math.max(6, 2 * scale);
    const x = Math.round(packet.at.x - size / 2);
    const y = Math.round(packet.at.y - size / 2);
    ctx.fillStyle = packet.color;
    ctx.fillRect(x, y, size, size);
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.fillRect(x + 1, y + 1, Math.max(2, size / 3), Math.max(2, size / 3));
  }
}
