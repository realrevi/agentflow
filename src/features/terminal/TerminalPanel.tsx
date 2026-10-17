// Lazy-loaded dock panel for one terminal (see layout/renderers.tsx). Default export.
// Agent panels add their pixel character (corner overlay) and a prompt bar.
import '@xterm/xterm/css/xterm.css';
import './terminal.css';
import type { DockviewPanelApi } from 'dockview-core';
import { createEffect, createSignal, onCleanup, onMount, Show, untrack } from 'solid-js';
import { reducedMotion } from '../characters';
import { settings } from '../settings/settingsStore';
import { themeVars } from '../theme/themeStore';
import { agentState, flashAgentState } from '../agentui';
import { budgetCovers, cliKeyOf, createPanelUsage, onBudgetAlert } from '../tokens';
import {
  AgentOverlay,
  createPromptHistory,
  playPacket,
  PromptBar,
  type AgentOverlayHandle,
  type PromptBarHandle,
  type SentPrompt,
} from '../agentui/components';
import { AgentActivity } from './agentActivity';
import {
  agentName,
  avatarFor,
  hasAgentUi,
  parseTerminalConfig,
  type TerminalConfig,
} from './config';
import { createDropTarget } from './dropTarget';
import { StartOverlay } from './StartOverlay';
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

const PACKET_MS = 600;

export default function TerminalPanel(props: TerminalPanelProps) {
  const [status, setStatus] = createSignal<TerminalStatus>('stopped');
  const [error, setError] = createSignal<string | null>(null);
  // The character can change while the panel is open (tab avatar / Characters view).
  const [config, setConfig] = createSignal(props.config);
  const [lastPrompt, setLastPrompt] = createSignal<{ text: string; at: number } | null>(null);
  let root: HTMLDivElement | undefined;
  let body: HTMLDivElement | undefined;
  let host: HTMLDivElement | undefined;
  let overlay: AgentOverlayHandle | undefined;
  let bar: PromptBarHandle | undefined;

  const agentUi = hasAgentUi(props.config.kind);
  const activity = new AgentActivity(props.panelId);
  // Agent CLIs: session usage on the badge; a budget alert for this CLI flashes "error"
  // (CLI agents can only be warned, never stopped).
  const cliKey = cliKeyOf(props.config.kind);
  const usage = cliKey ? createPanelUsage(props.panelId) : () => null;
  if (cliKey) {
    onCleanup(
      onBudgetAlert((status) => {
        if (budgetCovers(status.scope, cliKey)) flashAgentState(props.panelId, 'error');
      }),
    );
  }
  const controller = new TerminalController({
    panelId: props.panelId,
    config: props.config,
    activity: {
      start: () => activity.start(),
      output: () => activity.output(),
      enter: () => activity.prompt(),
      typed: () => activity.typed(),
      exit: (code) => activity.exit(code),
    },
    defaultCwd: () => settings.currentWorkspace,
    settings: untrack(() => ({ ...settings.terminal })),
    theme: untrack(() => xtermTheme(themeVars())),
    onStatus: (next, message) => {
      setStatus(next);
      setError(message);
      setTerminalStatus(props.panelId, next);
    },
  });
  const history = agentUi
    ? createPromptHistory(props.config.cwd, props.config.historyKey ?? props.panelId)
    : undefined;
  const character = () => avatarFor(config(), settings.characters);
  const drop = createDropTarget(
    () => status() === 'running',
    (text) => controller.insertText(text),
  );

  createEffect(() => controller.setTheme(xtermTheme(themeVars())));
  createEffect(() => controller.applySettings({ ...settings.terminal }));

  const send = (text: string) => {
    if (status() !== 'running') return false;
    activity.prompt(); // before the write, so its echo is not taken for work
    if (!controller.sendPrompt(text)) return false;
    history?.add(text);
    setLastPrompt({ text, at: Date.now() });
    return true;
  };
  const flyPacket = (sent: SentPrompt) => {
    const target = overlay;
    if (!target || !character()) return;
    if (!settings.characters.packet || reducedMotion() || !root) return target.receive();
    playPacket({
      host: root,
      ...sent,
      target: target.target(),
      duration: PACKET_MS / Math.max(0.25, settings.characters.speed),
      onArrive: () => target.receive(),
    });
  };

  onMount(() => {
    if (host) controller.attach(host);
    controller.setVisible(props.api.isVisible);
    if (props.api.isActive) controller.focus();
    const subscriptions = [
      props.api.onDidVisibilityChange((event) => controller.setVisible(event.isVisible)),
      props.api.onDidActiveChange((event) => event.isActive && controller.focus()),
      props.api.onDidParametersChange((params) => {
        const next = parseTerminalConfig((params as { config?: unknown }).config);
        if (next) setConfig(next);
      }),
    ];
    // The panel loads lazily: its parameters (e.g. the assigned character) may have changed
    // before it mounted.
    const current = parseTerminalConfig(props.api.getParameters<{ config?: unknown }>().config);
    if (current) setConfig(current);
    if (props.autoStart) void controller.start();
    onCleanup(() => {
      for (const subscription of subscriptions) subscription.dispose();
      controller.dispose();
      activity.dispose();
      clearTerminalStatus(props.panelId);
    });
  });

  return (
    <div
      ref={root}
      class="af-terminal"
      classList={{ dropping: drop.dropping() }}
      onDragOver={drop.onDragOver}
      onDragLeave={drop.onDragLeave}
      onDrop={drop.onDrop}
    >
      <div ref={body} class="af-terminal-body">
        <div ref={host} class="af-terminal-host" />
        <Show when={character()}>
          {(ref) => (
            <AgentOverlay
              ref={(handle) => (overlay = handle)}
              character={ref()}
              aiName={agentName(config())}
              state={agentState(props.panelId)}
              lastPrompt={lastPrompt()}
              history={history?.entries() ?? []}
              onPickPrompt={(text) => bar?.insert(text)}
              usage={usage()}
              cursor={() => controller.cursor()}
              onCursorChange={(listener) => {
                const subscription = controller.onCursorChange(listener);
                return () => subscription.dispose();
              }}
              host={() => host}
              body={() => body}
            />
          )}
        </Show>
      </div>
      <Show when={agentUi && history}>
        {(prompts) => (
          <PromptBar
            ref={(handle) => (bar = handle)}
            history={prompts()}
            character={character}
            enabled={() => status() === 'running'}
            onSend={send}
            onSent={flyPacket}
          />
        )}
      </Show>
      <Show when={status() === 'stopped'}>
        <StartOverlay config={config()} error={error()} onStart={() => void controller.start()} />
      </Show>
    </div>
  );
}
