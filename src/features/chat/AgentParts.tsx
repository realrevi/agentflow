// Agent pieces of the conversation as plain collapsed lines: reasoning, tool calls (with
// their results once they ran) and notices. Phase 7B replaces these with the polished UI.
import { For, Show } from 'solid-js';
import type { ChatMessage, Part, ToolCall, ToolResult } from '../../shared/types';
import { t } from '../i18n';
import { statusText, type AgentNotice, type LiveTool } from './agentTurn';

type Of<K extends Part['type']> = Extract<Part, { type: K }>;

export function reasoningOf(message: ChatMessage): string {
  return (message.parts ?? [])
    .filter((part): part is Of<'reasoning'> => part.type === 'reasoning')
    .map((part) => part.text)
    .join('\n')
    .trim();
}

export function callsOf(message: ChatMessage): ToolCall[] {
  return (message.parts ?? []).filter((part): part is Of<'toolCall'> => part.type === 'toolCall');
}

/** Tool results by call id, from the `tool` messages. */
export function resultsOf(messages: readonly ChatMessage[]): Map<string, ToolResult> {
  const results = new Map<string, ToolResult>();
  for (const message of messages) {
    if (message.role !== 'tool') continue;
    for (const part of message.parts ?? []) {
      if (part.type === 'toolResult') results.set(part.callId, part);
    }
  }
  return results;
}

export function Reasoning(props: { text: string; live?: boolean }) {
  return (
    <details class="af-chat-reasoning" open={props.live}>
      <summary>{t(props.live ? 'agent.reasoning.live' : 'agent.reasoning')}</summary>
      <pre>{props.text}</pre>
    </details>
  );
}

/** `ok: null` = still running. */
export function ToolLine(props: { call: ToolCall; ok: boolean | null; detail: string | null }) {
  const mark = () => (props.ok === null ? '…' : props.ok ? '✓' : '✗');
  return (
    <details
      class="af-chat-tool"
      classList={{ running: props.ok === null, failed: props.ok === false }}
    >
      <summary>
        <span class="af-chat-tool-mark" aria-hidden="true">
          {mark()}
        </span>{' '}
        {statusText(props.call)}
      </summary>
      <Show when={props.detail}>{(detail) => <pre>{detail()}</pre>}</Show>
    </details>
  );
}

/** The calls of a stored step: its result once stored, else the live state while it runs. */
export function MessageCalls(props: {
  message: ChatMessage;
  results: Map<string, ToolResult>;
  live: readonly LiveTool[];
}) {
  return (
    <For each={callsOf(props.message)}>
      {(call) => {
        const result = () => props.results.get(call.id);
        const live = () => props.live.find((tool) => tool.call.id === call.id);
        const ok = () => {
          const stored = result();
          return stored ? !stored.isError : (live()?.ok ?? null);
        };
        return (
          <ToolLine call={call} ok={ok()} detail={result()?.content ?? live()?.summary ?? null} />
        );
      }}
    </For>
  );
}

export function Notices(props: { notices: readonly AgentNotice[] }) {
  return (
    <For each={props.notices}>
      {(notice) => (
        <p class="af-chat-notice" title={notice.detail}>
          {t(
            notice.kind === 'maxIterations' ? 'agent.notice.maxIterations' : 'agent.notice.noTools',
          )}
        </p>
      )}
    </For>
  );
}
