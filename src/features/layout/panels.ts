// Panel kinds shown in the dock. Panel params are persisted inside the layout JSON
// (terminal config only, never output), so they are parsed defensively on restore.
import { parseTerminalConfig, terminalTitle, type TerminalConfig } from '../terminal';
import { t } from '../i18n';

export type PanelKind = 'terminal' | 'workspace' | 'settings';

export type PanelParams =
  | { kind: 'terminal'; index: number; config: TerminalConfig }
  | { kind: 'workspace' }
  | { kind: 'settings' };

const KINDS: readonly PanelKind[] = ['terminal', 'workspace', 'settings'];

export function isPanelKind(value: unknown): value is PanelKind {
  return typeof value === 'string' && (KINDS as readonly string[]).includes(value);
}

export function parsePanelParams(raw: unknown): PanelParams | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const { kind, index, config } = raw as Record<string, unknown>;
  if (kind === 'workspace' || kind === 'settings') return { kind };
  if (kind !== 'terminal') return null;
  const parsed = parseTerminalConfig(config);
  const validIndex = typeof index === 'number' && Number.isInteger(index) && index > 0;
  return parsed && validIndex ? { kind, index, config: parsed } : null;
}

/** Singletons (workspace, settings) use their kind as id; terminals are `terminal-<n>`. */
export function panelId(params: PanelParams): string {
  return params.kind === 'terminal' ? `terminal-${params.index}` : params.kind;
}

/** Translated tab title; recomputed when the language changes. */
export function panelTitle(params: PanelParams): string {
  switch (params.kind) {
    case 'terminal':
      return terminalTitle(params.config);
    case 'workspace':
      return t('panel.workspace.title');
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
  tabComponent?: string;
}

export const TERMINAL_TAB = 'terminal';

export function panelOptions(params: PanelParams): PanelOptions {
  const options: PanelOptions = {
    id: panelId(params),
    component: params.kind,
    title: panelTitle(params),
    params,
  };
  if (params.kind === 'terminal') options.tabComponent = TERMINAL_TAB;
  return options;
}
