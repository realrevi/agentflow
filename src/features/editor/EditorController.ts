// One editor: a CodeMirror view plus the file behind it. Imperative on purpose:
// EditorPanel.tsx wires it to Solid state (like TerminalController).
import { Compartment, EditorState, type Text } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { toAppError } from '../../shared/errors';
import { ipc } from '../../shared/ipc';
import { reportError } from '../i18n';
import { baseName } from '../workspace';
import { languageFor } from './languages';
import { loadLanguage } from './loadLanguage';
import { baseExtensions } from './setup';
import { detectLineSeparator, minimalChange } from './text';

export type EditorStatus = 'loading' | 'ready' | 'binary' | 'tooLarge' | 'error';
/** Why the file on disk and the editor disagree. */
export type DiskBanner = 'changed' | 'conflict' | 'deleted' | null;

export interface EditorInfo {
  status: EditorStatus;
  /** Size in bytes, for the binary / too large info views. */
  size?: number;
  message?: string;
  /** Not valid UTF-8: opened read-only so saving cannot destroy the original bytes. */
  readOnly?: boolean;
}

export interface EditorOptions {
  host: HTMLElement;
  path: string;
  dark: boolean;
  /** False while another workspace is open: no reads or writes then. */
  canUseFiles: () => boolean;
  onState: (state: EditorInfo) => void;
  onBanner: (banner: DiskBanner) => void;
  onDirty: (dirty: boolean) => void;
}

export class EditorController {
  private view: EditorView | undefined;
  private path: string;
  private dark: boolean;
  /** mtime of the file version the buffer is based on (`null`: no file on disk). */
  private mtime: number | null = null;
  /** The document as last loaded or saved; dirty means the buffer differs from it. */
  private saved: Text | null = null;
  private dirty = false;
  private saving = false;
  private recheck = false;
  /** True while a disk version is being applied (it never makes the buffer dirty). */
  private applyingDisk = false;
  private disposed = false;
  private readonly language = new Compartment();
  private readonly darkMode = new Compartment();

  constructor(private readonly options: EditorOptions) {
    this.path = options.path;
    this.dark = options.dark;
  }

  fileName(): string {
    return baseName(this.path);
  }

  async load(): Promise<void> {
    if (!this.options.canUseFiles()) return;
    this.options.onState({ status: 'loading' });
    try {
      const file = await ipc.fsRead(this.path);
      if (this.disposed) return;
      if (file.binary || file.content === null) {
        this.options.onState({ status: 'binary', size: file.size });
        return;
      }
      this.mtime = file.mtime;
      this.createView(file.content, file.lossy);
      this.options.onState({ status: 'ready', readOnly: file.lossy });
    } catch (err) {
      if (this.disposed) return;
      const error = toAppError(err);
      if (error.kind === 'tooLarge') this.options.onState({ status: 'tooLarge', size: error.size });
      else this.options.onState({ status: 'error', message: error.message });
    }
  }

  /** Resolves `true` once the file is saved. */
  async save(): Promise<boolean> {
    const view = this.view;
    if (!view || this.saving || view.state.readOnly || !this.options.canUseFiles()) return false;
    this.saving = true;
    const state = view.state;
    try {
      // `sliceDoc` joins lines with the file's own line separator.
      const stat = await ipc.fsWrite(this.path, state.sliceDoc(), this.mtime);
      this.mtime = stat.mtime;
      this.saved = state.doc;
      this.updateDirty(view.state.doc); // edits typed during the save stay dirty
      this.options.onBanner(null);
      return true;
    } catch (err) {
      if (toAppError(err).kind === 'conflict') this.options.onBanner('conflict');
      else reportError('error.fileSave', err);
      return false;
    } finally {
      this.saving = false;
      if (this.recheck) {
        this.recheck = false;
        void this.checkDisk();
      }
    }
  }

  /** The watcher reported a change to this file (or lost events). */
  async checkDisk(): Promise<void> {
    if (this.saving) {
      this.recheck = true;
      return;
    }
    if (!this.view) {
      // Not open as text (error, binary, too large): the new version may open fine.
      await this.load();
      return;
    }
    if (!this.options.canUseFiles()) return;
    try {
      const file = await ipc.fsRead(this.path);
      if (this.disposed || file.mtime === this.mtime) return;
      if (file.content === null) return;
      if (!this.dirty) this.replace(file.content, file.mtime);
      else this.options.onBanner('changed');
    } catch (err) {
      if (!this.disposed && toAppError(err).kind === 'notFound') this.options.onBanner('deleted');
    }
  }

  /** Banner action: take the version on disk. */
  async reload(): Promise<void> {
    try {
      const file = await ipc.fsRead(this.path);
      if (this.disposed || file.content === null) return;
      this.replace(file.content, file.mtime);
      this.options.onBanner(null);
      this.focus();
    } catch (err) {
      if (toAppError(err).kind === 'notFound') this.options.onBanner('deleted');
      else reportError('error.fileOpen', err);
    }
  }

  /** Banner action: keep the buffer; the next save overwrites (or recreates) the file. */
  async keepMine(): Promise<void> {
    try {
      this.mtime = (await ipc.fsRead(this.path)).mtime;
    } catch {
      this.mtime = null; // deleted: saving creates it again
      this.saved = null;
      if (this.view) this.updateDirty(this.view.state.doc);
    }
    this.options.onBanner(null);
    this.focus();
  }

  setPath(path: string): void {
    const languageChanged = languageFor(path) !== languageFor(this.path);
    this.path = path;
    if (languageChanged) void this.applyLanguage();
  }

  setDark(dark: boolean): void {
    this.dark = dark;
    this.view?.dispatch({ effects: this.darkMode.reconfigure(EditorView.darkTheme.of(dark)) });
  }

  /** Re-measure after the font size or visibility changed. */
  refresh(): void {
    this.view?.requestMeasure();
  }

  focus(): void {
    this.view?.focus();
  }

  dispose(): void {
    this.disposed = true;
    this.view?.destroy();
    this.view = undefined;
  }

  private createView(content: string, readOnly: boolean): void {
    const state = EditorState.create({
      doc: content,
      extensions: [
        baseExtensions(() => void this.save()),
        // Keep CRLF files CRLF (and mixed files byte-identical) when saving.
        EditorState.lineSeparator.of(detectLineSeparator(content)),
        EditorState.readOnly.of(readOnly),
        this.language.of([]),
        this.darkMode.of(EditorView.darkTheme.of(this.dark)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !this.applyingDisk) this.updateDirty(update.state.doc);
        }),
      ],
    });
    this.saved = state.doc;
    this.view = new EditorView({ state, parent: this.options.host });
    void this.applyLanguage();
  }

  /** Swaps in the disk version with a minimal change, so cursor and scroll stay put. */
  private replace(content: string, mtime: number): void {
    const view = this.view;
    if (!view) return;
    const change = minimalChange(view.state.sliceDoc(), content, lineBreak(view.state));
    this.applyingDisk = true;
    try {
      if (change) view.dispatch({ changes: change });
    } finally {
      this.applyingDisk = false;
    }
    this.mtime = mtime;
    this.saved = view.state.doc;
    this.updateDirty(view.state.doc);
  }

  private updateDirty(doc: Text): void {
    const saved = this.saved;
    const dirty = !saved || doc.length !== saved.length || !doc.eq(saved);
    if (dirty === this.dirty) return;
    this.dirty = dirty;
    this.options.onDirty(dirty);
  }

  private async applyLanguage(): Promise<void> {
    const path = this.path;
    try {
      const support = await loadLanguage(languageFor(path));
      if (this.disposed || path !== this.path) return;
      this.view?.dispatch({ effects: this.language.reconfigure(support) });
    } catch (err) {
      console.debug('[agentflow] language load failed:', err);
    }
  }
}

function lineBreak(state: EditorState): '\n' | '\r\n' {
  return state.lineBreak === '\r\n' ? '\r\n' : '\n';
}
