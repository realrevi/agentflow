// Lazy-loaded dock panel for one terminal (see layout/renderers.tsx). Default export.
import '@xterm/xterm/css/xterm.css';
import './terminal.css';
import type { DockviewPanelApi } from 'dockview-core';
import { createEffect, createSignal, onCleanup, onMount, Show, untrack } from 'solid-js';
import { FILE_PATH_MIME } from '../../shared/dnd';
import { isWindows } from '../../shared/platform';
import { agentsIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { themeVars } from '../theme/themeStore';
import { isAgentKind, terminalTitle, type TerminalConfig } from './config';
import { quotePath } from './quotePath';
import { clearTerminalStatus, setTerminalStatus } from './statusStore';
import { TerminalController } from './TerminalController';
import type { TerminalStatus } from './types';
import { xtermTheme } from './xtermTheme';

export interface TerminalPanelProps {
  panelId: string;
  config: TerminalConfig;
  api: DockviewPanelApi;
  /** Start right away (new panels, and restored shells when the setting is on). */
  autoStart: boolean;
}

export default function TerminalPanel(props: TerminalPanelProps) {
  const [status, setStatus] = createSignal<TerminalStatus>('stopped');
  const [error, setError] = createSignal<string | null>(null);
  const [dropping, setDropping] = createSignal(false);
  let host: HTMLDivElement | undefined;

  const controller = new TerminalController({
    config: props.config,
    defaultCwd: () => settings.currentWorkspace,
    settings: untrack(() => ({ ...settings.terminal })),
    theme: untrack(() => xtermTheme(themeVars())),
    onStatus: (next, message) => {
      setStatus(next);
      setError(message);
      setTerminalStatus(props.panelId, next);
    },
  });

  createEffect(() => controller.setTheme(xtermTheme(themeVars())));
  createEffect(() => controller.applySettings({ ...settings.terminal }));

  onMount(() => {
    if (host) controller.attach(host);
    controller.setVisible(props.api.isVisible);
    if (props.api.isActive) controller.focus();
    const subscriptions = [
      props.api.onDidVisibilityChange((event) => controller.setVisible(event.isVisible)),
      props.api.onDidActiveChange((event) => event.isActive && controller.focus()),
    ];
    if (props.autoStart) void controller.start();
    onCleanup(() => {
      for (const subscription of subscriptions) subscription.dispose();
      controller.dispose();
      clearTerminalStatus(props.panelId);
    });
  });

  // A file dragged from the tree types its quoted relative path at the cursor.
  const acceptsDrop = (event: DragEvent) =>
    status() === 'running' && (event.dataTransfer?.types.includes(FILE_PATH_MIME) ?? false);
  const onDragOver = (event: DragEvent) => {
    if (!acceptsDrop(event)) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    setDropping(true);
  };
  const onDrop = (event: DragEvent) => {
    setDropping(false);
    const path = event.dataTransfer?.getData(FILE_PATH_MIME);
    if (!acceptsDrop(event) || !path) return;
    event.preventDefault();
    controller.insertText(quotePath(path, isWindows()));
  };

  const commandLine = () =>
    props.config.kind === 'custom' ? [props.config.command, ...props.config.args].join(' ') : '';

  return (
    <div
      class="af-terminal"
      classList={{ dropping: dropping() }}
      onDragOver={onDragOver}
      onDragLeave={() => setDropping(false)}
      onDrop={onDrop}
    >
      <div ref={host} class="af-terminal-host" />
      <Show when={status() === 'stopped'}>
        <div class="af-terminal-overlay">
          <PixelIcon art={agentsIcon} scale={3} />
          <h2 class="af-pixel-title">{terminalTitle(props.config)}</h2>
          <Show when={commandLine()}>
            <code class="af-terminal-command">{commandLine()}</code>
          </Show>
          <Show when={error()}>{(message) => <p class="af-terminal-error">{message()}</p>}</Show>
          <button type="button" class="af-button" onClick={() => void controller.start()}>
            {t('terminal.start')}
          </button>
          <Show when={isAgentKind(props.config.kind)}>
            <p class="af-terminal-hint">{t('terminal.agentHint')}</p>
          </Show>
        </div>
      </Show>
    </div>
  );
}
