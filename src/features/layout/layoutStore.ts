// Owns the live dockview instance: restore/persist the layout, open and reset panels,
// keep tab titles translated, and expose the open panel list to the sidebar.
import type { DockviewApi, SerializedDockview } from 'dockview-core';
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { debounce, type Debounced } from '../../shared/debounce';
import { showToast } from '../../shared/toast';
import type { SerializedLayout } from '../../shared/types';
import { locale, reportError, t } from '../i18n';
import { saveLayout, savedLayout } from '../settings/settingsStore';
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

function nextAgentIndex(dock: DockviewApi): number {
  let index = 1;
  while (dock.getPanel(`agent-${index}`)) index++;
  return index;
}

/** Opens a panel (or focuses an existing singleton) in the given or active group. */
export function openPanel(kind: PanelKind, groupId?: string): void {
  if (!api) return;
  if (kind !== 'agent') {
    const existing = api.getPanel(kind);
    if (existing) {
      existing.api.setActive();
      return;
    }
  }
  const index = kind === 'agent' ? nextAgentIndex(api) : 1;
  const referenceGroup = groupId ?? api.activeGroup?.id;
  api.addPanel({
    ...panelOptions({ kind, index }),
    ...(referenceGroup ? { position: { referenceGroup } } : {}),
  });
}

export function focusPanel(id: string): void {
  api?.getPanel(id)?.api.setActive();
}

/** Restores the default panels and keeps Settings open, since that is where reset lives. */
export function resetLayout(): void {
  if (!api) return;
  api.clear();
  buildDefaultLayout(api);
  openPanel('settings');
  showToast({ kind: 'success', title: t('settings.layout.resetDone') });
}
