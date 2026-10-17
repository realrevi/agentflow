// The agent part of a running turn: live reasoning, the tool calls of the current step, the
// pending permission request and notices. Finished steps arrive as stored messages
// (`message` events) and render from their parts; this only holds what is in flight.
import { batch, createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { ChatEvent, PermissionAnswer, PermissionRequest, ToolCall } from '../../shared/types';
import { reportError, t, type I18nKey } from '../i18n';

export interface LiveTool {
  call: ToolCall;
  /** `null` while it runs. */
  ok: boolean | null;
  summary: string | null;
}

export interface AgentNotice {
  kind: 'toolsUnsupported' | 'maxIterations';
  detail: string;
}

type AgentEvent = Extract<
  ChatEvent,
  { type: 'reasoning' | 'status' | 'toolStart' | 'toolEnd' | 'permission' | 'notice' }
>;

const STATUS: Record<string, I18nKey> = {
  list_dir: 'agent.status.list',
  read_file: 'agent.status.read',
  glob: 'agent.status.glob',
  search: 'agent.status.search',
  write_file: 'agent.status.write',
  edit_file: 'agent.status.edit',
  run_command: 'agent.status.run',
};

export function isAgentEvent(event: ChatEvent): event is AgentEvent {
  return ['reasoning', 'status', 'toolStart', 'toolEnd', 'permission', 'notice'].includes(
    event.type,
  );
}

/** The main argument of a call, for one-line labels ("src/app.ts", "npm test"). */
export function callTarget(call: ToolCall): string {
  const input = call.input;
  for (const key of ['command', 'path', 'pattern']) {
    const value = input[key];
    if (typeof value === 'string' && value) return value;
  }
  return '.';
}

/** "Reading src/app.ts", "Running npm test". */
export function statusText(call: ToolCall): string {
  const key = STATUS[call.name];
  return key ? t(key, { target: callTarget(call) }) : call.name;
}

export function createAgentTurn(onPhase: (phase: 'think' | 'work' | 'wait') => void) {
  const [reasoning, setReasoning] = createSignal('');
  const [tools, setTools] = createSignal<readonly LiveTool[]>([]);
  const [permission, setPermission] = createSignal<PermissionRequest | null>(null);
  const [notices, setNotices] = createSignal<readonly AgentNotice[]>([]);
  const [status, setStatus] = createSignal<string | null>(null);

  function handle(event: AgentEvent): void {
    switch (event.type) {
      case 'reasoning':
        setReasoning((text) => text + event.text);
        break;
      case 'status':
        onPhase(event.phase);
        if (event.phase === 'think') setStatus(null);
        break;
      case 'toolStart':
        batch(() => {
          setTools((list) => [...list, { call: event.call, ok: null, summary: null }]);
          setStatus(statusText(event.call));
        });
        onPhase('work');
        break;
      case 'toolEnd':
        setTools((list) =>
          list.map((tool) =>
            tool.call.id === event.callId
              ? { ...tool, ok: event.ok, summary: event.summary }
              : tool,
          ),
        );
        break;
      case 'permission':
        setPermission(event.request);
        onPhase('wait');
        break;
      case 'notice':
        setNotices((list) => [...list, { kind: event.kind, detail: event.message }]);
        break;
    }
  }

  /** A step was stored: its live parts now render from the message. */
  function stepStored(): void {
    batch(() => {
      setReasoning('');
      setTools([]);
    });
  }

  async function answer(value: PermissionAnswer): Promise<void> {
    const request = permission();
    if (!request) return;
    setPermission(null);
    try {
      await ipc.agentPermissionAnswer(request.id, value);
    } catch (err) {
      reportError('error.agentAnswer', err);
    }
  }

  function reset(keepNotices = false): void {
    batch(() => {
      stepStored();
      setPermission(null);
      setStatus(null);
      if (!keepNotices) setNotices([]);
    });
  }

  return { reasoning, tools, permission, notices, status, handle, stepStored, answer, reset };
}

export type AgentTurn = ReturnType<typeof createAgentTurn>;
