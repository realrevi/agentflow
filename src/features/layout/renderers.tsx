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
import {
  createEffect,
  createSignal,
  lazy,
  Match,
  onCleanup,
  Show,
  Suspense,
  Switch,
  type JSX,
} from 'solid-js';
import { clearAttention, glowing, interactionClears } from '../agentui';
import type { StageHooks } from '../stage/hooks';
import { openNewWorkstream } from '../workstreams/dialogState';
import type { ChatHooks } from '../chat/agent/agentView';
import { FilesPanel, markAgentChanged } from '../files';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { AddPanelButton } from './AddPanelButton';
import { currentWorkspace } from '../workspace';
import { setChatTitle } from './chatPanels';
import { editorFileDirty, openEditor, renameEditors } from './editorPanels';
import { consumeFreshPanel, currentDock, focusPanel, openPanel, openPanels } from './layoutStore';
import { moveStage, openTimeline, stageIsLeft } from './stagePanels';
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
const DiffView = lazy(() => import('../editor/DiffView'));
const InstallerPanel = lazy(() => import('../agents/InstallerPanel'));
const StagePanel = lazy(() => import('../stage/StagePanel'));
const TimelinePanel = lazy(() => import('../workstreams/TimelinePanel'));

/** The Stage reads panels and moves through these (it cannot import the layout). */
const stageHooks: StageHooks = {
  panels: openPanels,
  focus: focusPanel,
  openTimeline: (workstream, pair) => openTimeline(workstream, pair),
  newWorkstream: () => openNewWorkstream(),
  move: moveStage,
  isLeft: stageIsLeft,
  history: (panelId) => {
    const params = parsePanelParams(currentDock()?.getPanel(panelId)?.params);
    if (params?.kind === 'terminal') {
      return { workspace: params.config.cwd, key: params.config.historyKey ?? panelId };
    }
    if (params?.kind === 'chat') {
      return { workspace: params.config.workspace, key: params.config.historyKey ?? panelId };
    }
    return null;
  },
};

/** Chat panels open files, check unsaved edits, mark agent changes and show diffs. */
const chatHooks: ChatHooks = {
  openFile: openEditor,
  isFileDirty: editorFileDirty,
  filesChanged: (paths) => {
    const root = currentWorkspace();
    if (root) markAgentChanged(root, paths);
  },
  Diff: DiffView,
  setTitle: setChatTitle,
};

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
        <Match when={params.kind === 'installer'}>
          <InstallerPanel hooks={{ Diff: DiffView, openFile: openEditor }} />
        </Match>
        <Match when={params.kind === 'stage'}>
          <StagePanel api={props.api} hooks={stageHooks} />
        </Match>
        <Match when={params.kind === 'timeline' && params}>
          {(timeline) => (
            <TimelinePanel
              api={props.api}
              params={{ workstream: timeline().workstream, pair: timeline().pair }}
            />
          )}
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
          {(chat) => (
            <ChatPanel
              panelId={props.id}
              config={chat().config}
              api={props.api}
              hooks={chatHooks}
            />
          )}
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

const INTERACTIONS = ['pointerdown', 'keydown', 'wheel'] as const;

export function createContentRenderer(options: CreateComponentOptions): IContentRenderer {
  const part = createSolidPart('af-panel');
  let removeListeners: (() => void) | undefined;
  return {
    element: part.element,
    init(init) {
      const params = parsePanelParams(init.params);
      if (!params) {
        console.warn(`[agentflow] unknown panel "${options.id}" (${options.name})`);
        return;
      }
      part.mount(() => {
        // The attention glow on the panel's frame, cleared by focus + interaction.
        createEffect(() => part.element.classList.toggle('af-glow-frame', glowing(options.id)));
        return <PanelView id={options.id} params={params} api={init.api} />;
      });
      const clear = (event: Event) => {
        if (interactionClears(event.type, init.api.isActive)) clearAttention(options.id);
      };
      for (const type of INTERACTIONS) {
        part.element.addEventListener(type, clear, { capture: true, passive: true });
      }
      removeListeners = () => {
        for (const type of INTERACTIONS) part.element.removeEventListener(type, clear, true);
      };
    },
    dispose: () => {
      removeListeners?.();
      part.dispose();
    },
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
