// Types shared with the Rust backend. Keep in sync with src-tauri/src/{settings,themes,error}.

export type Locale = 'en' | 'tr' | 'de' | 'zh' | 'ru';
export type SidebarView = 'agents' | 'explorer' | 'characters';
export type ThemeKind = 'dark' | 'light';

export interface AgentBoldSettings {
  color: string;
  glow: boolean;
}

export interface AppearanceSettings {
  themeId: string;
  /** `null` means "use the theme's color". */
  textColor: string | null;
  accentColor: string | null;
  agentBold: AgentBoldSettings;
}

export interface UiSettings {
  sidebarVisible: boolean;
  sidebarWidth: number;
  activeView: SidebarView;
}

/** Opaque dockview layout JSON; only the layout feature interprets it. */
export type SerializedLayout = Record<string, unknown>;

export type CursorStyle = 'block' | 'underline' | 'bar';

export interface TerminalSettings {
  fontSize: number;
  scrollback: number;
  cursorStyle: CursorStyle;
  /** Restored shell panels start on launch; agent CLIs never do. */
  autoStartShells: boolean;
}

export interface FilesSettings {
  /** Show dotfiles in the file tree and quick open (`.git` always stays hidden). */
  showHidden: boolean;
}

export interface Settings {
  version: number;
  language: Locale;
  appearance: AppearanceSettings;
  ui: UiSettings;
  /** Folder new terminals start in; `null` means the home directory. */
  currentWorkspace: string | null;
  /** Most recent first, at most 10. */
  recentWorkspaces: string[];
  terminal: TerminalSettings;
  files: FilesSettings;
  layout: SerializedLayout | null;
}

export interface ThemeSummary {
  id: string;
  name: string;
  kind: ThemeKind;
  builtin: boolean;
}

export interface ThemeData extends ThemeSummary {
  /** VS Code `colors` entries, already validated as hex colors by the backend. */
  colors: Record<string, string>;
}

export type PtyKind = 'shell' | 'claude' | 'codex' | 'gemini' | 'custom';

// A type alias (not an interface) so it is assignable to `invoke`'s argument record.
export type PtySpawnRequest = {
  kind: PtyKind;
  /** Custom terminals: the program, or a whole command line when `args` is empty. */
  command: string | null;
  args: string[];
  /** `null` starts in the home directory. */
  cwd: string | null;
  cols: number;
  rows: number;
};

export interface SessionInfo {
  id: number;
  kind: PtyKind;
  program: string;
  args: string[];
  cwd: string;
  pid: number | null;
}

/** The last message of every session. */
export interface PtyExitMessage {
  type: 'exit';
  code: number | null;
  signal: string | null;
}

/** Raw output bytes, or the exit message. */
export type PtyMessage = ArrayBuffer | PtyExitMessage;

export interface CliStatus {
  kind: PtyKind;
  binary: string;
  found: boolean;
  path: string | null;
}

// Workspace file system (src-tauri/src/fs). Paths are relative to the workspace root
// and always use `/`.

export interface FsEntry {
  name: string;
  path: string;
  isDir: boolean;
  isSymlink: boolean;
}

export interface FileContent {
  /** `null` for binary files. */
  content: string | null;
  binary: boolean;
  /** Not valid UTF-8: invalid bytes were replaced with U+FFFD. */
  lossy: boolean;
  size: number;
  /** Modification time (ms since the epoch); pass it back to `fs_write`. */
  mtime: number;
}

export interface FileStat {
  size: number;
  mtime: number;
}

export type EntryKind = 'file' | 'dir';

export interface QuickOpenMatch {
  path: string;
  /** Matched character (code point) indices, for highlighting. */
  positions: number[];
}

/** One debounced batch of watcher events. */
export interface FsChanges {
  paths: string[];
  /** Created, removed or renamed: their parent folder needs a re-list. */
  structural: string[];
  /** Ignore rules changed or events were lost: re-list everything. */
  rescan: boolean;
}

export type AppErrorKind =
  | 'io'
  | 'json'
  | 'invalidTheme'
  | 'notFound'
  | 'invalidInput'
  | 'pty'
  | 'conflict'
  | 'tooLarge'
  | 'internal';
