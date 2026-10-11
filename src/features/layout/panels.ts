// Panel kinds shown in the dock. Panel params are persisted inside the layout JSON
// (terminal config and editor file paths, never output or file contents), so they are
// parsed defensively on restore.
import { chatTitle, parseChatConfig, type ChatConfig } from '../chat/config';
import { parseTerminalConfig, terminalTitle, type TerminalConfig } from '../terminal';
import { t } from '../i18n';
import { baseName } from '../workspace';

export type PanelKind = 'terminal' | 'chat' | 'editor' | 'files' | 'settings';

export type PanelParams =
  | { kind: 'terminal'; index: number; config: TerminalConfig }
  /** An API chat: one stored conversation. */
  | { kind: 'chat'; index: number; config: ChatConfig }
  /** `root` is the workspace folder the file belongs to; `path` is relative to it. */
  | { kind: 'editor'; index: number; root: string; path: string }
  | { kind: 'files' }
  | { kind: 'settings' };

export type SingletonKind = 'files' | 'settings';

function isIndex(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0;
}

function isText(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

export function parsePanelParams(raw: unknown): PanelParams | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const { kind, index, config, root, path } = raw as Record<string, unknown>;
  if (kind === 'files' || kind === 'settings') return { kind };
  if (kind === 'editor') {
    return isIndex(index) && isText(root) && isText(path) ? { kind, index, root, path } : null;
  }
  if (kind === 'chat') {
    const chat = parseChatConfig(config);
    return chat && isIndex(index) ? { kind, index, config: chat } : null;
  }
  if (kind !== 'terminal') return null;
  const parsed = parseTerminalConfig(config);
  return parsed && isIndex(index) ? { kind, index, config: parsed } : null;
}

/** Singletons use their kind as id; the others are `<kind>-<n>` (e.g. `chat-2`). */
export function panelId(params: PanelParams): string {
  return 'index' in params ? `${params.kind}-${params.index}` : params.kind;
}

/** Translated tab title; recomputed when the language changes. */
export function panelTitle(params: PanelParams): string {
  switch (params.kind) {
    case 'terminal':
      return terminalTitle(params.config);
    case 'chat':
      return chatTitle(params.config);
    case 'editor':
      return baseName(params.path);
    case 'files':
      return t('panel.files.title');
    case 'settings':
      return t('panel.settings.title');
  }
}

/** The base of every `addPanel` call; callers spread in a position. */
export interface PanelOptions {
  id: string;
  component: PanelKind;
  title: string;
  params: PanelParams;
  tabComponent: string;
}

/** One tab renderer for every panel (status dot, dirty dot, guarded close). */
export const PANEL_TAB = 'panel';

export function panelOptions(params: PanelParams): PanelOptions {
  return {
    id: panelId(params),
    component: params.kind,
    title: panelTitle(params),
    params,
    tabComponent: PANEL_TAB,
  };
}
