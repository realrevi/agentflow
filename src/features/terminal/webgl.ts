// The WebGL renderer is fast, but browsers cap live WebGL contexts (~16), so only visible
// terminals hold one: attach when shown, detach when hidden. Without WebGL (or after a
// context loss) xterm keeps rendering with its default DOM renderer.
import { WebglAddon } from '@xterm/addon-webgl';
import type { Terminal } from '@xterm/xterm';

const MAX_FAILURES = 3;
let webgl2: boolean | undefined;

/** Probes WebGL2 once, releasing the probe context right away. */
function webgl2Supported(): boolean {
  if (webgl2 === undefined) {
    try {
      const gl = document.createElement('canvas').getContext('webgl2');
      webgl2 = gl !== null;
      gl?.getExtension('WEBGL_lose_context')?.loseContext();
    } catch {
      webgl2 = false;
    }
  }
  return webgl2;
}

/** Frees a context now instead of whenever the canvas is garbage collected. */
function loseContext(canvas: HTMLCanvasElement): void {
  try {
    canvas.getContext('webgl2')?.getExtension('WEBGL_lose_context')?.loseContext();
  } catch {
    // Already lost or never a WebGL canvas.
  }
}

export class WebglRenderer {
  private addon: WebglAddon | undefined;
  private canvases: HTMLCanvasElement[] = [];
  private failures = 0;

  constructor(private readonly term: Terminal) {}

  attach(): void {
    if (this.addon || this.failures >= MAX_FAILURES || !webgl2Supported()) return;
    try {
      const before = new Set(this.term.element?.querySelectorAll('canvas'));
      const addon = new WebglAddon();
      addon.onContextLoss(() => {
        this.failures++;
        this.detach();
      });
      this.term.loadAddon(addon);
      this.addon = addon;
      // Remember the canvases the addon created, to release their contexts on detach.
      this.canvases = [...(this.term.element?.querySelectorAll('canvas') ?? [])].filter(
        (canvas) => !before.has(canvas),
      );
    } catch (err) {
      this.failures++;
      console.warn('[agentflow] WebGL renderer unavailable, using the DOM renderer', err);
    }
  }

  /**
   * Disposing the addon does not release its WebGL context until GC, and browsers evict the
   * oldest live context once too many exist, so the context is lost explicitly.
   */
  detach(): void {
    const addon = this.addon;
    const canvases = this.canvases;
    this.addon = undefined;
    this.canvases = [];
    addon?.dispose();
    canvases.forEach(loseContext);
  }
}
