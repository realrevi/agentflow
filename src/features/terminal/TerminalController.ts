// One terminal panel: an xterm.js instance plus the pty session behind it.
// Imperative on purpose: TerminalPanel.tsx wires it to Solid state.
import { FitAddon } from '@xterm/addon-fit';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { Terminal, type IDisposable, type ITheme } from '@xterm/xterm';
import { toAppError } from '../../shared/errors';
import { ipc, type PtyHandle } from '../../shared/ipc';
import type { PtyMessage, TerminalSettings } from '../../shared/types';
import { reportError, t } from '../i18n';
import { copySelection, pasteInto } from './clipboard';
import type { TerminalConfig } from './config';
import { OutputDecoder } from './decoder';
import { terminalShortcut } from './keys';
import { encodePrompt } from './prompt/encode';
import { AckBatcher, InputQueue, LatestQueue } from './queues';
import { FONT_FAMILY, fontsReady, isExit, RESET_MODES, type CursorCell } from './support';
import type { TerminalStatus } from './types';
import { WebglRenderer } from './webgl';

const FIT_DEBOUNCE_MS = 50;
/** Acknowledge processed output in 64 KB steps (backend pauses at 512 KB unacknowledged). */
const ACK_CHUNK = 64 * 1024;

interface Session {
  id: number;
  handle: PtyHandle;
  input: InputQueue;
  resize: LatestQueue<{ cols: number; rows: number }>;
}

/** One start of the process: its exit state and flow-control bookkeeping. */
interface Launch {
  exited: boolean;
  sessionId: number | undefined;
  acks: AckBatcher;
}

/** Activity signals for the agent state heuristic (agentActivity.ts). */
export interface ActivityHooks {
  start(): void;
  output(): void;
  /** Enter typed directly into the terminal. */
  enter(): void;
  exit(code: number | null): void;
}

export interface ControllerOptions {
  config: TerminalConfig;
  activity?: ActivityHooks;
  /** Used when the config has no folder: the current workspace (null = home). */
  defaultCwd: () => string | null;
  settings: TerminalSettings;
  theme: ITheme;
  onStatus: (status: TerminalStatus, error: string | null) => void;
}

const quiet = (err: unknown) => console.debug('[agentflow] terminal IPC:', err);

export class TerminalController {
  private readonly term: Terminal;
  private readonly fit = new FitAddon();
  private readonly webgl: WebglRenderer;
  private readonly subscriptions: IDisposable[] = [];
  private host: HTMLElement | undefined;
  private observer: ResizeObserver | undefined;
  private fitTimer: number | undefined;
  private opened = false;
  private visible = false;
  private wantsFocus = false;
  private disposed = false;
  private status: TerminalStatus = 'stopped';
  private session: Session | undefined;

  constructor(private readonly options: ControllerOptions) {
    const { settings, theme } = options;
    this.term = new Terminal({
      fontFamily: FONT_FAMILY,
      fontSize: settings.fontSize,
      scrollback: settings.scrollback,
      cursorStyle: settings.cursorStyle,
      cursorBlink: true,
      theme,
      allowProposedApi: true, // required by the unicode11 addon
    });
    this.term.loadAddon(this.fit);
    this.term.loadAddon(new Unicode11Addon());
    this.term.unicode.activeVersion = '11';
    this.webgl = new WebglRenderer(this.term);
    this.term.attachCustomKeyEventHandler((event) => this.onKey(event));
    this.subscriptions.push(
      this.term.onData((data) => this.onInput(data)),
      this.term.onResize((size) => this.session?.resize.push(size)),
    );
  }

  /** xterm opens once `host` has a real size, i.e. it is attached and visible. */
  attach(host: HTMLElement): void {
    this.host = host;
    this.observer = new ResizeObserver(() => this.scheduleFit());
    this.observer.observe(host);
  }

  setVisible(visible: boolean): void {
    this.visible = visible;
    if (!this.opened) return;
    if (visible) {
      this.webgl.attach();
      this.scheduleFit();
    } else {
      this.webgl.detach();
    }
  }

  focus(): void {
    if (this.opened) this.term.focus();
    else this.wantsFocus = true;
  }

  setTheme(theme: ITheme): void {
    this.term.options.theme = theme;
  }

  applySettings(settings: TerminalSettings): void {
    const options = this.term.options;
    if (options.fontSize !== settings.fontSize) options.fontSize = settings.fontSize;
    if (options.scrollback !== settings.scrollback) options.scrollback = settings.scrollback;
    if (options.cursorStyle !== settings.cursorStyle) options.cursorStyle = settings.cursorStyle;
    this.scheduleFit();
  }

  /** Types `text` at the cursor (bracketed if the program asked for it), without Enter. */
  insertText(text: string): void {
    if (this.status !== 'running' || this.disposed) return;
    this.term.paste(text);
    this.term.focus();
  }

  /**
   * Sends a prompt as if typed: multiline text inside bracketed paste when the program
   * enabled it, then Enter. Written to the pty immediately. Returns false when not running.
   */
  sendPrompt(text: string): boolean {
    if (this.status !== 'running' || !this.session) return false;
    this.session.input.push(encodePrompt(text, this.term.modes.bracketedPasteMode));
    return true;
  }

  /** The cursor in visible-screen cells (public xterm API only). */
  cursor(): CursorCell | null {
    if (!this.opened) return null;
    const buffer = this.term.buffer.active;
    const row = buffer.cursorY + buffer.baseY - buffer.viewportY;
    return { col: buffer.cursorX, row, cols: this.term.cols, rows: this.term.rows };
  }

  /** Calls `listener` when the cursor moves, the view scrolls or the size changes. */
  onCursorChange(listener: () => void): IDisposable {
    const subscriptions = [
      this.term.onCursorMove(listener),
      this.term.onScroll(listener),
      this.term.onResize(listener),
    ];
    return { dispose: () => subscriptions.forEach((subscription) => subscription.dispose()) };
  }

  /** Spawns the pty (no-op while starting or running). */
  async start(): Promise<void> {
    if (this.disposed || this.status === 'starting' || this.status === 'running') return;
    this.setStatus('starting', null);
    const launch: Launch = { exited: false, sessionId: undefined, acks: new AckBatcher(ACK_CHUNK) };
    const decoder = new OutputDecoder();
    const { config } = this.options;
    try {
      const handle = await ipc.ptySpawn(
        {
          kind: config.kind,
          command: config.command,
          args: config.args,
          cwd: config.cwd ?? this.options.defaultCwd(),
          cols: this.term.cols,
          rows: this.term.rows,
        },
        (message) => this.onMessage(launch, decoder, message),
      );
      if (this.disposed) {
        handle.detach();
        void ipc.ptyKill(handle.info.id).catch(quiet);
        return;
      }
      if (launch.exited) {
        handle.detach(); // it already finished; onMessage has shown the exit line
        return;
      }
      const id = handle.info.id;
      launch.sessionId = id;
      this.acknowledge(launch, 0); // output may have been processed before the id was known
      this.session = {
        id,
        handle,
        input: new InputQueue((data) => ipc.ptyWrite(id, data), quiet),
        resize: new LatestQueue((size) => ipc.ptyResize(id, size.cols, size.rows), quiet),
      };
      this.session.resize.push({ cols: this.term.cols, rows: this.term.rows });
      this.setStatus('running', null);
      this.options.activity?.start();
    } catch (err) {
      if (this.disposed) return;
      this.setStatus('stopped', toAppError(err).message);
      reportError('error.terminalStart', err);
    }
  }

  /** Kills the session and frees xterm, its addons and observers. */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    window.clearTimeout(this.fitTimer);
    this.observer?.disconnect();
    for (const subscription of this.subscriptions) subscription.dispose();
    const session = this.session;
    this.session = undefined;
    if (session) {
      session.handle.detach();
      void ipc.ptyKill(session.id).catch(quiet);
    }
    this.webgl.detach();
    this.term.dispose();
  }

  private setStatus(status: TerminalStatus, error: string | null): void {
    if (this.disposed) return;
    this.status = status;
    this.options.onStatus(status, error);
  }

  private onMessage(launch: Launch, decoder: OutputDecoder, message: PtyMessage): void {
    // Closed while starting: the channel is still attached until spawn returns.
    if (this.disposed) return;
    if (message instanceof ArrayBuffer) {
      this.options.activity?.output();
      const bytes = message.byteLength;
      this.term.write(decoder.decode(new Uint8Array(message)), () =>
        this.acknowledge(launch, bytes),
      );
      return;
    }
    if (!isExit(message)) return;
    launch.exited = true;
    const tail = decoder.flush();
    if (tail) this.term.write(tail);
    this.term.write(RESET_MODES);
    this.session?.handle.detach();
    this.session = undefined;
    const line =
      message.code === null
        ? t('terminal.exitedNoCode')
        : t('terminal.exited', { code: message.code });
    this.term.write(`\r\n\x1b[2m${line}\x1b[22m\r\n`);
    this.setStatus('exited', null);
    this.options.activity?.exit(message.code);
  }

  /** Reports output xterm has finished processing, so the backend keeps reading. */
  private acknowledge(launch: Launch, bytes: number): void {
    const id = launch.sessionId;
    const ready = launch.acks.add(bytes, id !== undefined && !launch.exited);
    if (id !== undefined && ready > 0) void ipc.ptyAck(id, ready).catch(quiet);
  }

  private onInput(data: string): void {
    if (this.status === 'running') {
      this.session?.input.push(data);
      if (data.includes('\r')) this.options.activity?.enter();
    } else if (this.status === 'exited' && data === '\r') void this.start();
  }

  /** Returns false for keys AgentFlow handles itself, so xterm ignores them. */
  private onKey(event: KeyboardEvent): boolean {
    const shortcut = terminalShortcut(event);
    if (!shortcut) return true;
    if (shortcut !== 'newShell' && event.type === 'keydown') {
      event.preventDefault();
      if (shortcut === 'copy') void copySelection(this.term);
      else void pasteInto(this.term, () => this.disposed);
    }
    return false;
  }

  private scheduleFit(): void {
    window.clearTimeout(this.fitTimer);
    this.fitTimer = window.setTimeout(() => void this.fitNow(), FIT_DEBOUNCE_MS);
  }

  private async fitNow(): Promise<void> {
    const host = this.host;
    if (!host || this.disposed || host.clientWidth === 0 || host.clientHeight === 0) return;
    if (!this.opened) {
      await fontsReady();
      if (this.disposed || this.opened) return;
      this.term.open(host);
      this.opened = true;
      if (this.visible) this.webgl.attach();
      if (this.wantsFocus) this.term.focus();
    }
    this.fit.fit(); // emits onResize -> pty_resize when cols/rows changed
  }
}
