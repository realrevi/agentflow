// What a running agent turn has in flight: live reasoning (with its timing, so it can
// collapse when the answer starts), tool calls that started or ended but are not stored
// yet, the pending permission request and notices. Stored steps render from messages.
import { batch, createSignal } from 'solid-js';
import { createStore, reconcile } from 'solid-js/store';
import { ipc } from '../../../shared/ipc';
import type {
  AgentPhase,
  ChatEvent,
  PermissionAnswer,
  PermissionRequest,
  ToolResult,
} from '../../../shared/types';
import { reportError, t } from '../../i18n';
import { activeText } from './toolText';

export interface LiveTool {
  startedAt: number | null;
  result: ToolResult | null;
}

export interface LiveReasoning {
  text: string;
  startedAt: number;
  /** When the answer (text or a tool call) started: the section collapses. */
  endedAt: number | null;
}

export interface AgentNotice {
  kind: 'toolsUnsupported' | 'maxIterations';
  detail: string;
}

type AgentEvent = Extract<
  ChatEvent,
  { type: 'reasoning' | 'status' | 'toolStart' | 'toolEnd' | 'permission' | 'notice' }
>;

const AGENT_EVENTS: ReadonlySet<ChatEvent['type']> = new Set([
  'reasoning',
  'status',
  'toolStart',
  'toolEnd',
  'permission',
  'notice',
]);

export function isAgentEvent(event: ChatEvent): event is AgentEvent {
  return AGENT_EVENTS.has(event.type);
}

export interface LiveTurnHooks {
  phase(phase: AgentPhase): void;
  /** A permission request arrived (the panel toasts when it is not visible). */
  asked(request: PermissionRequest): void;
}

export function createLiveTurn(
  conversationId: string,
  hooks: LiveTurnHooks,
  now: () => number = Date.now,
) {
  const [reasoning, setReasoning] = createSignal<LiveReasoning | null>(null);
  const [tools, setTools] = createStore<Record<string, LiveTool>>({});
  const [permission, setPermission] = createSignal<PermissionRequest | null>(null);
  const [notices, setNotices] = createSignal<readonly AgentNotice[]>([]);
  const [status, setStatus] = createSignal<string | null>(null);

  function handle(event: AgentEvent): void {
    switch (event.type) {
      case 'reasoning':
        setReasoning((live) =>
          live
            ? { ...live, text: live.text + event.text }
            : { text: event.text, startedAt: now(), endedAt: null },
        );
        break;
      case 'status':
        hooks.phase(event.phase);
        if (event.phase === 'think') setStatus(null);
        break;
      case 'toolStart':
        batch(() => {
          answerStarted();
          setTools(event.call.id, { startedAt: now(), result: null });
          setStatus(activeText(event.call));
        });
        break;
      case 'toolEnd':
        // Denied and skipped calls end here too, so their ✗ shows at once.
        setTools(event.result.callId, (tool) => ({
          startedAt: tool?.startedAt ?? null,
          result: event.result,
        }));
        if (permission()?.callId === event.result.callId) setPermission(null);
        break;
      case 'permission':
        setPermission(event.request);
        hooks.asked(event.request);
        break;
      case 'notice':
        setNotices((list) => [...list, { kind: event.kind, detail: event.message }]);
        break;
    }
  }

  /** The first answer text or tool call of a step: live reasoning collapses. */
  function answerStarted(): void {
    setReasoning((live) => (live && live.endedAt === null ? { ...live, endedAt: now() } : live));
  }

  /** A stored step arrived: its reasoning (or tool results) now render from messages. */
  function stepStored(role: 'assistant' | 'tool' | 'user'): void {
    if (role === 'assistant') setReasoning(null);
    if (role === 'tool') setTools(reconcile({}));
  }

  /**
   * Answers request `requestId`, only from a real click or keypress on its own card
   * (`event.isTrusted`) and only while that request is the pending one.
   */
  async function answer(requestId: number, value: PermissionAnswer, event: Event): Promise<void> {
    const request = permission();
    if (!event.isTrusted || !request || request.id !== requestId) return;
    setPermission(null);
    try {
      await ipc.agentPermissionAnswer(request.id, conversationId, value);
    } catch (err) {
      reportError('error.agentAnswer', err);
    }
  }

  function reset(keepNotices = false): void {
    batch(() => {
      setReasoning(null);
      setTools(reconcile({}));
      setPermission(null);
      setStatus(null);
      if (!keepNotices) setNotices([]);
    });
  }

  /** The prompt "Continue" sends after the step limit. */
  const continuePrompt = () => t('agent.continuePrompt');

  return {
    reasoning,
    tool: (callId: string): LiveTool | undefined => tools[callId],
    permission,
    notices,
    status,
    handle,
    answerStarted,
    stepStored,
    answer,
    reset,
    continuePrompt,
  };
}

export type LiveTurn = ReturnType<typeof createLiveTurn>;
