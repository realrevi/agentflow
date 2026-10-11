// Owns the live dockview instance: restore/persist the layout, open and reset panels,
// keep tab titles translated, and expose the open panel list to the sidebar.
import type { DockviewApi, SerializedDockview } from 'dockview-core';
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { debounce, type Debounced } from '../../shared/debounce';
import { showToast } from '../../shared/toast';
import type { SerializedLayout } from '../../shared/types';
import { locale, reportError, t } from '../i18n';
import { saveLayout, savedLayout } from '../settings/settingsStore';
import type { TerminalConfig } from '../terminal';
import { buildDefaultLayout } from './defaultLayout';
import { panelOptions, panelTitle, parsePanelParams, type PanelKind } from './panels';

export interface OpenPanel {
  id: string;
  title: string;
  kind: PanelKind;
  active: boolean;
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

  // dockview's JSON is plain data; the settings file stores it as an opaque object.
  const persistLayout = debounce(
    () => saveLayout(dock.toJSON() as unknown as SerializedLayout),
    PERSIST_DELAY_MS,
  );
  persist = persistLayout;
  const subscriptions = [
    dock.onDidLayoutChange(persistLayout),
    dock.onDidAddPanel(syncOpenPanels),
    dock.onDidRemovePanel(syncOpenPanels),
    dock.onDidActivePanelChange(syncOpenPanels),
  ];
  const disposeEffects = createRoot((dispose) => {
    createEffect(on(locale, retitlePanels, { defer: true }));
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
  const saved = savedLayout();
  if (saved) {
    try {
      dock.fromJSON(saved as unknown as SerializedDockview);
      retitlePanels();
      return;
    } catch (err) {
      reportError('error.layoutRestore', err);
      dock.clear();
    }
  }
  buildDefaultLayout(dock);
}

function syncOpenPanels(): void {
  const activeId = api?.activePanel?.id;
  const panels = (api?.panels ?? []).flatMap((panel): OpenPanel[] => {
    const params = parsePanelParams(panel.params);
    if (!params) return [];
    return [
      { id: panel.id, title: panelTitle(params), kind: params.kind, active: panel.id === activeId },
    ];
  });
  setOpenPanels(panels);
}

function retitlePanels(): void {
  for (const panel of api?.panels ?? []) {
    const params = parsePanelParams(panel.params);
    if (params) panel.api.setTitle(panelTitle(params));
  }
  syncOpenPanels();
}

function nextTerminalIndex(dock: DockviewApi): number {
  let index = 1;
  while (dock.getPanel(`terminal-${index}`)) index++;
  return index;
}

/** True once for panels opened in this session; restored panels return false. */
export function consumeFreshPanel(id: string): boolean {
  return freshPanels.delete(id);
}

function referencePosition(dock: DockviewApi, groupId?: string) {
  const referenceGroup = groupId ?? dock.activeGroup?.id;
  return referenceGroup ? { position: { referenceGroup } } : {};
}

/** Opens (or focuses) the Workspace or Settings panel in the given or active group. */
export function openPanel(kind: Exclude<PanelKind, 'terminal'>, groupId?: string): void {
  if (!api) return;
  const existing = api.getPanel(kind);
  if (existing) {
    existing.api.setActive();
    return;
  }
  api.addPanel({ ...panelOptions({ kind }), ...referencePosition(api, groupId) });
}

/** Opens a new terminal panel; it starts right away. */
export function openTerminal(config: TerminalConfig, groupId?: string): void {
  if (!api) return;
  const options = panelOptions({ kind: 'terminal', index: nextTerminalIndex(api), config });
  freshPanels.add(options.id);
  api.addPanel({ ...options, ...referencePosition(api, groupId) });
}

export function focusPanel(id: string): void {
  api?.getPanel(id)?.api.setActive();
}

/** Restores the default panels and keeps Settings open, since that is where reset lives. */
export function resetLayout(): void {
  if (!api) return;
  api.clear(); // disposes every panel, which kills its terminal session
  buildDefaultLayout(api);
  openPanel('settings');
  showToast({ kind: 'success', title: t('settings.layout.resetDone') });
}
