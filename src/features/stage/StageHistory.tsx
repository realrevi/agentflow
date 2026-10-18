// Double-click on a Stage character: the prompts sent from its panel.
import { Show } from 'solid-js';
import { createPromptHistory, HistoryPopover } from '../agentui/components';
import type { StageHooks } from './hooks';
import type { Point } from './scene';

export function StageHistory(props: {
  panel: string;
  at: Point;
  hooks: StageHooks;
  onClose: () => void;
}) {
  let anchor: HTMLDivElement | undefined;
  const where = props.hooks.history(props.panel);
  const entries = where ? createPromptHistory(where.workspace, where.key).entries : () => [];
  return (
    <div
      ref={anchor}
      class="af-stage-history"
      style={{ left: `${props.at.x}px`, top: `${props.at.y}px` }}
    >
      <Show when={anchor}>
        {(element) => (
          <HistoryPopover
            entries={entries()}
            anchor={element()}
            onPick={() => props.hooks.focus(props.panel)}
            onClose={props.onClose}
          />
        )}
      </Show>
    </div>
  );
}
