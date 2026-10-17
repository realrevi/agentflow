// The in-flight part of an agent turn under the stored messages: live reasoning, the
// reply streaming right now and the pending permission card.
import { Show, type JSX } from 'solid-js';
import { settings } from '../../settings/settingsStore';
import type { AgentView } from './agentView';
import { PermissionCard } from './PermissionCard';
import { ReasoningBlock } from './ReasoningBlock';

export function LiveSection(props: { view: AgentView; reply: (busy: boolean) => JSX.Element }) {
  const live = () => props.view.session.live;
  /** Reasoning streams or a permission waits: no empty reply caret. */
  const busy = () => Boolean(live().permission() || live().reasoning());
  return (
    <>
      <Show when={settings.agent.showReasoning && live().reasoning()}>
        {(reasoning) => (
          <ReasoningBlock
            text={reasoning().text}
            view={{
              live: true,
              answering: reasoning().endedAt !== null,
              durationMs:
                reasoning().endedAt === null
                  ? undefined
                  : (reasoning().endedAt ?? 0) - reasoning().startedAt,
            }}
          />
        )}
      </Show>
      {props.reply(busy())}
      <Show when={live().permission()}>
        {(request) => (
          <PermissionCard
            request={request()}
            workspace={props.view.workspace}
            timeoutSecs={settings.agent.commandTimeoutSecs}
            isDirty={props.view.isFileDirty}
            Diff={props.view.Diff}
            onAnswer={(id, answer, event) => void live().answer(id, answer, event)}
          />
        )}
      </Show>
    </>
  );
}
