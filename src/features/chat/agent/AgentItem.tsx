// Timeline items of agent turns: reasoning, a tool card, a group of reads, a turn's end.
import { Match, Show, Switch } from 'solid-js';
import { settings } from '../../settings/settingsStore';
import type { AgentView } from './agentView';
import { ReasoningBlock } from './ReasoningBlock';
import type { TimelineItem } from './timeline';
import { ToolCard } from './ToolCard';
import { ToolGroup } from './ToolGroup';
import { TurnFooter } from './TurnFooter';

export function AgentItem(props: { item: TimelineItem; view: AgentView }) {
  const live = (callId: string) => props.view.session.live.tool(callId);
  const checkpoints = () => props.view.session.checkpoints;
  const item = props.item;
  return (
    <Switch>
      <Match when={item.kind === 'reasoning' && item}>
        {(reasoning) => (
          <Show when={settings.agent.showReasoning}>
            <ReasoningBlock
              text={reasoning().text}
              view={{ live: false, answering: true, durationMs: reasoning().durationMs }}
            />
          </Show>
        )}
      </Match>
      <Match when={item.kind === 'tool' && item}>
        {(tool) => (
          <ToolCard call={tool().call} result={tool().result} live={live(tool().call.id)} />
        )}
      </Match>
      <Match when={item.kind === 'group' && item}>
        {(group) => <ToolGroup tools={group().tools} live={live} />}
      </Match>
      <Match when={item.kind === 'turnEnd' && item}>
        {(end) => (
          <Show when={checkpoints().byTurn(end().turn)}>
            {(checkpoint) => (
              <TurnFooter
                checkpoint={checkpoint()}
                undoing={checkpoints().undoing() === end().turn}
                onOpen={props.view.openFile}
                onUndo={() => void checkpoints().undo(end().turn)}
              />
            )}
          </Show>
        )}
      </Match>
    </Switch>
  );
}
