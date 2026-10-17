// Owns the live dockview instance: restore/persist the layout (with group roles), open
// panels in the right group, guard closing, reset, and keep tab titles translated.
import type { DockviewApi, DockviewGroupPanel } from 'dockview-core';
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { askChoice } from '../../shared/dialog';
import { debounce, type Debounced } from '../../shared/debounce';
import { showToast } from '../../shared/toast';
import type { CharacterRef } from '../characters';
import { confirmEditorClose, dirtyEditorCount } from '../editor';
import { locale, reportError, t } from '../i18n';
import { saveLayout, savedLayout, settings } from '../settings/settingsStore';
import type { TerminalConfig } from '../terminal';
import {
  characterOf,
  ensureCharacters,
  forgetCharacter,
  setCharacter,
  trackFocusedAgent,
} from './characters';
import { buildDefaultLayout } from './defaultLayout';
import {
  panelOptions,
  panelTitle,
  parsePanelParams,
  type PanelKind,
  type PanelParams,
  type SingletonKind,
} from './panels';
import { createRoleGroup, groupForRole } from './placement';
import {
  forgetGroup,
  groupRole,
  noteGroupFocus,
  replaceRoles,
  roleForPanel,
  setGroupRole,
  snapshotRoles,
} from './roles';
import { parseSavedLayout, serializeLayout } from './savedLayout';

export interface OpenPanel {
  id: string;
  title: string;
  kind: PanelKind;
  active: boolean;
  /** The pixel character shown for it (agent panels; respects the avatar settings). */
  character: CharacterRef | null;
}

const PERSIST_DELAY_MS = 400;

let api: DockviewApi | undefined;
let persist: Debounced | undefined;
/** Panels created in this session (vs. restored from the saved layout). */
const freshPanels = new Set<string>();
const [openPanels, setOpenPanels] = createSignal<readonly OpenPanel[]>([]);

export { openPanels };

/** Connects a freshly created dockview. Returns a cleanup function. */
export function attachLayout(dock: DockviewApi): () => void {
  api = dock;
  restore(dock);
  assignCharacters();

  const persistLayout = debounce(
    () => saveLayout(serializeLayout(dock.toJSON(), snapshotRoles())),
    PERSIST_DELAY_MS,
  );
  persist = persistLayout;
  const subscriptions = [
    // Also catches retitled panels (a chat names itself after its first prompt).
    dock.onDidLayoutChange(() => {
      persistLayout();
      syncOpenPanels();
    }),
    dock.onDidAddPanel((panel) => {
      ensureRole(panel.group);
      syncOpenPanels();
    }),
    // Moves between groups fire no add/remove: only closed (or cleared) panels end here.
    dock.onDidRemovePanel((panel) => {
      forgetCharacter(panel.id);
      trackFocusedAgent(dock);
      syncOpenPanels();
    }),
    dock.onDidActivePanelChange(() => {
      trackFocusedAgent(dock);
      syncOpenPanels();
    }),
    dock.onDidActiveGroupChange((group) => group && noteGroupFocus(group.id)),
    // A group made by dragging a panel to an edge takes the role of that panel.
    dock.onDidAddGroup((group) => queueMicrotask(() => ensureRole(group))),
    dock.onDidMovePanel((event) => ensureRole(event.panel.group)),
    dock.onDidRemoveGroup((group) => forgetGroup(group.id)),
  ];
  const disposeEffects = createRoot((dispose) => {
    createEffect(on(locale, retitlePanels, { defer: true }));
    const avatars = () => [settings.characters.enabled, settings.characters.onShells];
    createEffect(on(avatars, assignCharacters, { defer: true }));
    return dispose;
  });
  syncOpenPanels();

  return () => {
    persistLayout.flush();
    persist = undefined;
    disposeEffects();
    for (const subscription of subscriptions) subscription.dispose();
    api = undefined;
  };
}

/** Writes a pending layout change right away (used when the window is closing). */
export function flushLayout(): void {
  persist?.flush();
}

function restore(dock: DockviewApi): void {
  const saved = parseSavedLayout(savedLayout());
  if (saved) {
    try {
      replaceRoles(saved.roles);
      dock.fromJSON(saved.dock);
      for (const group of dock.groups) ensureRole(group);
      retitlePanels();
      return;
    } catch (err) {
      reportError('error.layoutRestore', err);
      dock.clear();
    }
  }
  replaceRoles({});
  buildDefaultLayout(dock);
}

function ensureRole(group: DockviewGroupPanel | undefined): void {
  if (!group || groupRole(group.id)) return;
  const params = group.panels[0] ? parsePanelParams(group.panels[0].params) : null;
  setGroupRole(group.id, params ? roleForPanel(params) : 'agents');
}

export function syncOpenPanels(): void {
  const activeId = api?.activePanel?.id;
  const panels = (api?.panels ?? []).flatMap((panel): OpenPanel[] => {
    const params = parsePanelParams(panel.params);
    if (!params) return [];
    const character = characterOf(params);
    const title = panelTitle(params);
    return [{ id: panel.id, title, kind: params.kind, active: panel.id === activeId, character }];
  });
  if (!samePanels(openPanels(), panels)) setOpenPanels(panels);
}

/** Layout changes fire often (e.g. while dragging a sash): only real changes re-render. */
function samePanels(a: readonly OpenPanel[], b: readonly OpenPanel[]): boolean {
  return (
    a.length === b.length &&
    a.every((panel, i) => {
      const other = b[i];
      return (
        other !== undefined &&
        panel.id === other.id &&
        panel.title === other.title &&
        panel.active === other.active &&
        panel.character?.id === other.character?.id &&
        panel.character?.palette === other.character?.palette
      );
    })
  );
}

/** Gives agent panels without one a character (sprites load on first use). */
export function assignCharacters(): void {
  const dock = api;
  if (!dock) return;
  ensureCharacters(dock)
    .then(() => {
      if (api !== dock) return;
      trackFocusedAgent(dock);
      syncOpenPanels();
    })
    .catch((err: unknown) => reportError('error.characters', err));
}

/** Changes an agent panel's character (persists with the layout). */
export function setPanelCharacter(id: string, character: CharacterRef): void {
  if (api && setCharacter(api, id, character)) syncOpenPanels();
}

function retitlePanels(): void {
  for (const panel of api?.panels ?? []) {
    const params = parsePanelParams(panel.params);
    if (params) panel.api.setTitle(panelTitle(params));
  }
  syncOpenPanels();
}

/** The live dock (for the panel helpers in editorPanels.ts and chatPanels.ts). */
export function currentDock(): DockviewApi | undefined {
  return api;
}

export function nextIndex(dock: DockviewApi, kind: 'terminal' | 'chat' | 'editor'): number {
  let index = 1;
  while (dock.getPanel(`${kind}-${index}`)) index++;
  return index;
}

/** True once for panels opened in this session; restored panels return false. */
export function consumeFreshPanel(id: string): boolean {
  return freshPanels.delete(id);
}

/** Adds a panel to `groupId` (the "+" it was picked from) or the group of its role. */
export function addPanel(dock: DockviewApi, params: PanelParams, groupId?: string): void {
  const target =
    (groupId ? dock.getGroup(groupId) : undefined) ?? groupForRole(dock, roleForPanel(params));
  dock.addPanel({ ...panelOptions(params), position: { referenceGroup: target.id } });
}

/** Opens (or focuses) the Files or Settings panel. */
export function openPanel(kind: SingletonKind, groupId?: string): void {
  if (!api) return;
  const existing = api.getPanel(kind);
  if (existing) existing.api.setActive();
  else addPanel(api, { kind }, groupId);
}

/** Opens a new terminal panel; it starts right away. */
export function openTerminal(config: TerminalConfig, groupId?: string): void {
  if (!api) return;
  const index = nextIndex(api, 'terminal');
  freshPanels.add(`terminal-${index}`);
  addPanel(api, { kind: 'terminal', index, config }, groupId);
  assignCharacters();
}

export function focusPanel(id: string): void {
  api?.getPanel(id)?.api.setActive();
}

/**
 * Closes a panel from its tab. Dirty editors ask first. Closing the last panel of the
 * center keeps an (empty) agent area, so the layout does not collapse.
 */
export async function closePanel(id: string): Promise<void> {
  const panel = api?.getPanel(id);
  if (!api || !panel || !(await confirmEditorClose(id))) return;
  const group = panel.group;
  const lastInCenter =
    group.panels.length === 1 &&
    groupRole(group.id) === 'agents' &&
    api.groups.filter((other) => groupRole(other.id) === 'agents').length === 1;
  panel.api.close();
  if (lastInCenter && !api.getGroup(group.id)) createRoleGroup(api, 'agents').api.setActive();
}

/** Restores the default panels and keeps Settings open, since that is where reset lives. */
export async function resetLayout(): Promise<void> {
  if (!api) return;
  const dirty = dirtyEditorCount();
  if (dirty > 0) {
    const choice = await askChoice({
      title: t('layout.resetDirty.title'),
      message: t('layout.resetDirty.body', { count: dirty }),
      choices: [
        { id: 'reset', label: t('layout.resetDirty.discard'), kind: 'danger' },
        { id: 'cancel', label: t('dialog.cancel') },
      ],
      cancelId: 'cancel',
    });
    if (choice !== 'reset' || !api) return;
  }
  api.clear(); // disposes every panel, which kills its terminal session
  replaceRoles({});
  buildDefaultLayout(api);
  assignCharacters();
  openPanel('settings');
  showToast({ kind: 'success', title: t('settings.layout.resetDone') });
}
