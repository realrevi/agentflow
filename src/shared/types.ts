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

export interface Settings {
  version: number;
  language: Locale;
  appearance: AppearanceSettings;
  ui: UiSettings;
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

export type AppErrorKind = 'io' | 'json' | 'invalidTheme' | 'notFound' | 'internal';
