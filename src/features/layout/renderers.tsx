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
import { createSignal, lazy, Match, onCleanup, Show, Suspense, Switch, type JSX } from 'solid-js';
import { FilesPanel } from '../files';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { AddPanelButton } from './AddPanelButton';
import { openEditor, renameEditors } from './editorPanels';
import { consumeFreshPanel, openPanel, openPanels } from './layoutStore';
import { createPanelTab } from './PanelTab';
import { PANEL_TAB, parsePanelParams, type PanelParams } from './panels';
import { createSolidPart } from './solidPart';
import { Watermark } from './Watermark';

// Lazy: settings UI, xterm.js, CodeMirror and the chat (markdown, DOMPurify) are only
// downloaded when first needed.
const SettingsPanel = lazy(() => import('../settings/SettingsPanel'));
const TerminalPanel = lazy(() => import('../terminal/TerminalPanel'));
const ChatPanel = lazy(() => import('../chat/ChatPanel'));
const EditorPanel = lazy(() => import('../editor/EditorPanel'));
const UsagePanel = lazy(() => import('../tokens/UsagePanel'));

function Loading(): JSX.Element {
  return <div class="af-panel-loading">{t('panel.loading')}</div>;
}

/** Renders `children` only once the panel has been visible (restored background tabs
 * do not read their file or load CodeMirror until they are shown). */
function WhenVisible(props: { api: DockviewPanelApi; children: JSX.Element }) {
  const [seen, setSeen] = createSignal(props.api.isVisible);
  const subscription = props.api.onDidVisibilityChange((event) => {
    if (event.isVisible) setSeen(true);
  });
  onCleanup(() => subscription.dispose());
  return <Show when={seen()}>{props.children}</Show>;
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
      <Switch>
        <Match when={params.kind === 'settings'}>
          <SettingsPanel />
        </Match>
        <Match when={params.kind === 'usage'}>
          <UsagePanel
            panelTitle={(id) => openPanels().find((panel) => panel.id === id)?.title}
            onOpenSettings={() => openPanel('settings')}
          />
        </Match>
        <Match when={params.kind === 'files'}>
          <FilesPanel onOpenFile={openEditor} onRenamed={renameEditors} />
        </Match>
        <Match when={params.kind === 'editor' && params}>
          {(editor) => (
            <WhenVisible api={props.api}>
              <EditorPanel
                panelId={props.id}
                api={props.api}
                root={editor().root}
                path={editor().path}
              />
            </WhenVisible>
          )}
        </Match>
        <Match when={params.kind === 'chat' && params}>
          {(chat) => <ChatPanel panelId={props.id} config={chat().config} api={props.api} />}
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
  return options.name === PANEL_TAB ? createPanelTab() : undefined;
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
    init: (params) => part.mount(() => <Watermark groupId={params.group?.id} />),
    dispose: part.dispose,
  };
}
