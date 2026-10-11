// dockview renderer factories. Each one mounts a Solid tree into the element dockview gives us.
import type {
  CreateComponentOptions,
  DockviewGroupPanel,
  DockviewPanelApi,
  IContentRenderer,
  IHeaderActionsRenderer,
  ITabRenderer,
  IWatermarkRenderer,
} from 'dockview-core';
import { lazy, Match, Suspense, Switch, type JSX } from 'solid-js';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { AddPanelButton } from './AddPanelButton';
import { consumeFreshPanel } from './layoutStore';
import { parsePanelParams, TERMINAL_TAB, type PanelParams } from './panels';
import { PlaceholderPanel } from './PlaceholderPanel';
import { createSolidPart } from './solidPart';
import { createTerminalTab } from './TerminalTab';
import { Watermark } from './Watermark';

// Lazy: settings UI and xterm.js are only downloaded and parsed when first needed.
const SettingsPanel = lazy(() => import('../settings/SettingsPanel'));
const TerminalPanel = lazy(() => import('../terminal/TerminalPanel'));

function Loading(): JSX.Element {
  return <div class="af-panel-loading">{t('panel.loading')}</div>;
}

function PanelView(props: { id: string; params: PanelParams; api: DockviewPanelApi }) {
  const params = props.params;
  // New panels start right away; restored shells only if the setting says so, and
  // restored agent CLIs never (they cost tokens).
  const autoStart =
    params.kind === 'terminal' &&
    (consumeFreshPanel(props.id) ||
      (params.config.kind === 'shell' && settings.terminal.autoStartShells));
  return (
    <Suspense fallback={<Loading />}>
      <Switch fallback={<PlaceholderPanel />}>
        <Match when={params.kind === 'settings'}>
          <SettingsPanel />
        </Match>
        <Match when={params.kind === 'terminal' && params}>
          {(terminal) => (
            <TerminalPanel
              panelId={props.id}
              config={terminal().config}
              api={props.api}
              autoStart={autoStart}
            />
          )}
        </Match>
      </Switch>
    </Suspense>
  );
}

export function createContentRenderer(options: CreateComponentOptions): IContentRenderer {
  const part = createSolidPart('af-panel');
  return {
    element: part.element,
    init(init) {
      const params = parsePanelParams(init.params);
      if (!params) {
        console.warn(`[agentflow] unknown panel "${options.id}" (${options.name})`);
        return;
      }
      part.mount(() => <PanelView id={options.id} params={params} api={init.api} />);
    },
    dispose: part.dispose,
  };
}

export function createTabRenderer(options: CreateComponentOptions): ITabRenderer | undefined {
  return options.name === TERMINAL_TAB ? createTerminalTab() : undefined;
}

export function createHeaderActions(group: DockviewGroupPanel): IHeaderActionsRenderer {
  const part = createSolidPart('af-header-actions');
  return {
    element: part.element,
    init: () => part.mount(() => <AddPanelButton groupId={group.id} />),
    dispose: part.dispose,
  };
}

export function createWatermark(): IWatermarkRenderer {
  const part = createSolidPart('af-watermark-host');
  return {
    element: part.element,
    init: () => part.mount(() => <Watermark />),
    dispose: part.dispose,
  };
}
