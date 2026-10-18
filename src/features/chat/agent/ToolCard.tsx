// One tool call: pixel icon, one-line summary, status (running / ✓ / ✗ incl. denied at
// once) and, expanded, its input and output (capped, "show all").
import { createSignal, Match, Show, Switch } from 'solid-js';
import type { ToolCall, ToolResult } from '../../../shared/types';
import { Icon } from '../../../shared/ui/Icon';
import { t, type I18nKey } from '../../i18n';
import type { LiveTool } from './liveTurn';
import { toolIcon } from './toolIcons';
import { effectiveResult, toolState, toolSummary, type ToolState } from './toolText';
import { ToolDetails } from './ToolDetails';

export const STATE_LABEL: Record<ToolState, I18nKey> = {
  running: 'agent.tool.state.running',
  pending: 'agent.tool.state.pending',
  done: 'agent.tool.state.done',
  error: 'agent.tool.state.error',
  denied: 'agent.tool.state.denied',
  skipped: 'agent.tool.state.skipped',
};

export function StateMark(props: { state: ToolState }) {
  return (
    <span
      class={`af-tool-state ${props.state}`}
      role="img"
      aria-label={t(STATE_LABEL[props.state])}
      title={t(STATE_LABEL[props.state])}
    >
      <Switch fallback="✗">
        <Match when={props.state === 'running'}>
          <span class="af-tool-spinner" aria-hidden="true" />
        </Match>
        <Match when={props.state === 'pending'}>…</Match>
        <Match when={props.state === 'done'}>✓</Match>
      </Switch>
    </span>
  );
}

export function ToolCard(props: {
  call: ToolCall;
  result: ToolResult | undefined;
  live: LiveTool | undefined;
}) {
  const [open, setOpen] = createSignal(false);
  const result = () => effectiveResult(props.result, props.live);
  const state = () => toolState(result(), props.live?.startedAt != null);
  return (
    <div class={`af-tool-card ${state()}`}>
      <button
        type="button"
        class="af-tool-head"
        aria-expanded={open()}
        onClick={() => setOpen((value) => !value)}
      >
        <Icon name={toolIcon(props.call.name)} size={14} class="af-tool-icon" />
        <span class="af-tool-summary">{toolSummary(props.call, result())}</span>
        <Show when={state() === 'denied' || state() === 'skipped'}>
          <span class="af-tool-chip">{t(STATE_LABEL[state()])}</span>
        </Show>
        <StateMark state={state()} />
      </button>
      <Show when={open()}>
        <ToolDetails call={props.call} result={result()} />
      </Show>
    </div>
  );
}
