// Panel kinds shown in the dock. Panel params are persisted inside the layout JSON,
// so they are parsed defensively when a saved layout is restored.
import { t } from '../i18n';

export type PanelKind = 'agent' | 'workspace' | 'settings';

export interface PanelParams {
  kind: PanelKind;
  /** 1-based agent number; always 1 for singleton panels. */
  index: number;
}

const KINDS: readonly PanelKind[] = ['agent', 'workspace', 'settings'];

export function isPanelKind(value: unknown): value is PanelKind {
  return typeof value === 'string' && (KINDS as readonly string[]).includes(value);
}

export function parsePanelParams(raw: unknown): PanelParams | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const { kind, index } = raw as Record<string, unknown>;
  if (!isPanelKind(kind)) return null;
  const valid = typeof index === 'number' && Number.isInteger(index) && index > 0;
  return { kind, index: valid ? index : 1 };
}

/** Singletons (workspace, settings) use their kind as id, agents are `agent-<n>`. */
export function panelId(params: PanelParams): string {
  return params.kind === 'agent' ? `agent-${params.index}` : params.kind;
}

/** Translated tab title; recomputed when the language changes. */
export function panelTitle(params: PanelParams): string {
  switch (params.kind) {
    case 'agent':
      return t('panel.agent.title', { n: params.index });
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
}

export function panelOptions(params: PanelParams): PanelOptions {
  return { id: panelId(params), component: params.kind, title: panelTitle(params), params };
}
