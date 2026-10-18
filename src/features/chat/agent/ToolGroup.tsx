// Consecutive read-only calls as one "Explored N items" row; expanded, each call's card.
import { createSignal, For, Show } from 'solid-js';
import { Icon } from '../../../shared/ui/Icon';
import { t } from '../../i18n';
import type { LiveTool } from './liveTurn';
import type { ToolEntry } from './timeline';
import { StateMark, ToolCard } from './ToolCard';
import { effectiveResult, toolState, type ToolState } from './toolText';

/** Running wins, then failures; all done = done. */
export function groupState(states: readonly ToolState[]): ToolState {
  for (const state of ['running', 'pending', 'error', 'denied', 'skipped'] as const) {
    if (states.includes(state)) return state;
  }
  return 'done';
}

export function ToolGroup(props: {
  tools: readonly ToolEntry[];
  live: (callId: string) => LiveTool | undefined;
}) {
  const [open, setOpen] = createSignal(false);
  const state = () =>
    groupState(
      props.tools.map((entry) => {
        const live = props.live(entry.call.id);
        return toolState(effectiveResult(entry.result, live), live?.startedAt != null);
      }),
    );
  return (
    <div class={`af-tool-card af-tool-group ${state()}`}>
      <button
        type="button"
        class="af-tool-head"
        aria-expanded={open()}
        onClick={() => setOpen((value) => !value)}
      >
        <Icon name="folder-search" size={14} class="af-tool-icon" />
        <span class="af-tool-summary">
          {t('agent.group.explored', { count: props.tools.length })}
        </span>
        <StateMark state={state()} />
      </button>
      <Show when={open()}>
        <div class="af-tool-group-items">
          <For each={props.tools}>
            {(entry) => (
              <ToolCard call={entry.call} result={entry.result} live={props.live(entry.call.id)} />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
