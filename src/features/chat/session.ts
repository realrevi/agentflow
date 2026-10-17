// One chat panel's live state: the stored messages, the reply streaming right now (revealed
// smoothly on the shared animation loop), and the character's state: think on send, work on
// the first delta, done on stop (idle 5 s later), error on error.
import { batch, createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type {
  Attachment,
  ChatError,
  ChatEvent,
  ChatMessage,
  Conversation,
  TokenUsage,
} from '../../shared/types';
import { clearAgentState, setAgentState } from '../agentui';
import { reducedMotion, startAnimation, type AgentState } from '../characters';
import { reportError } from '../i18n';
import { settings } from '../settings/settingsStore';
import { RevealBuffer } from './reveal';

const REST_MS = 5000;

export interface Streaming {
  /** What is visible so far. */
  text: string;
  usage: TokenUsage | null;
}

export interface ChatSessionOptions {
  panelId: string;
  workspace: string | null;
  conversationId: string;
  onTitle: (title: string) => void;
  /** A reply ended: the conversation list should refresh. */
  onChanged: () => void;
}

type Ending = Extract<ChatEvent, { type: 'stop' | 'error' }>;

/** Retry drops failed replies at the end, as the backend does. */
export function withoutFailedTail(list: readonly ChatMessage[]): ChatMessage[] {
  const out = [...list];
  while (out.at(-1)?.role === 'assistant' && out.at(-1)?.status === 'error') out.pop();
  return out;
}

export function createChatSession(options: ChatSessionOptions) {
  const [conversation, setConversation] = createSignal<Conversation | null>(null);
  const [messages, setMessages] = createSignal<readonly ChatMessage[]>([]);
  const [streaming, setStreaming] = createSignal<Streaming | null>(null);
  const [error, setError] = createSignal<ChatError | null>(null);
  let requestId: number | null = null;
  let stopWanted = false;
  let detach: (() => void) | null = null;
  let reveal: RevealBuffer | null = null;
  let stopAnimation: (() => void) | undefined;
  let ending: Ending | null = null;
  let firstDelta = true;
  let restTimer: number | undefined;
  let disposed = false;

  const state = (next: AgentState) => {
    window.clearTimeout(restTimer);
    setAgentState(options.panelId, next);
    if (next === 'done') {
      restTimer = window.setTimeout(() => setAgentState(options.panelId, 'idle'), REST_MS);
    }
  };

  async function load(): Promise<void> {
    try {
      const loaded = await ipc.conversationGet(options.workspace, options.conversationId);
      if (disposed) return;
      batch(() => {
        setConversation(loaded);
        setMessages(loaded.messages);
      });
      options.onTitle(loaded.title);
    } catch (err) {
      reportError('error.chatLoad', err);
    }
  }

  function finish(): void {
    const end = ending;
    if (!end) return;
    ending = null;
    detach?.();
    detach = null;
    requestId = null;
    stopAnimation?.();
    stopAnimation = undefined;
    batch(() => {
      if (end.message) setMessages((list) => [...list, end.message as ChatMessage]);
      setStreaming(null);
      if (end.type === 'error') setError(end.error);
    });
    if (end.type === 'error') state('error');
    else state(end.reason === 'cancelled' ? 'idle' : 'done');
    options.onChanged();
  }

  /** Reveals on the shared loop; it stops as soon as everything is shown. */
  function animate(): void {
    stopAnimation ??= startAnimation({
      step(now) {
        if (!reveal) return false;
        const text = reveal.step(now);
        setStreaming((current) => current && { ...current, text });
        if (reveal.backlog > 0) return true;
        stopAnimation = undefined;
        if (ending) queueMicrotask(finish);
        return false;
      },
    });
  }

  function onEvent(event: ChatEvent): void {
    if (disposed) return;
    switch (event.type) {
      case 'delta':
        if (firstDelta) {
          firstDelta = false;
          state('work');
        }
        if (reveal) {
          reveal.push(event.text, performance.now());
          animate();
        } else {
          setStreaming((current) => current && { ...current, text: current.text + event.text });
        }
        break;
      case 'usage':
        setStreaming((current) => current && { ...current, usage: event.usage });
        break;
      default:
        ending = event;
        if (reveal && reveal.backlog > 0) animate();
        else finish();
    }
  }

  /** Sends a prompt (`text: null` = retry). Resolves false if it could not be sent. */
  async function send(text: string | null, attachments: Attachment[]): Promise<boolean> {
    if (streaming() || disposed) return false;
    firstDelta = true;
    stopWanted = false;
    ending = null;
    reveal = settings.chat.smoothReveal && !reducedMotion() ? new RevealBuffer() : null;
    batch(() => {
      setError(null);
      setStreaming({ text: '', usage: null });
      if (text === null) setMessages(withoutFailedTail);
    });
    state('think');
    try {
      const request = {
        workspace: options.workspace,
        conversationId: options.conversationId,
        text,
        attachments,
        panel: options.panelId,
        allowOnce: null,
      };
      const { started, detach: stopListening } = await ipc.chatSend(request, onEvent);
      if (!ending) {
        detach = stopListening;
        requestId = started.requestId;
      }
      const user = started.userMessage;
      if (user) setMessages((list) => [...list.filter((m) => m.id !== user.id), user].sort(byTime));
      if (started.title) options.onTitle(started.title);
      if (stopWanted && requestId !== null) void ipc.chatCancel(requestId);
      return true;
    } catch (err) {
      setStreaming(null);
      state('idle');
      reportError('error.chatSend', err);
      return false;
    }
  }

  function stop(): void {
    if (!streaming()) return;
    if (requestId === null) stopWanted = true;
    else ipc.chatCancel(requestId).catch((err: unknown) => reportError('error.chatSend', err));
  }

  /** A failed reply, or a prompt that never got one, can be retried. */
  function canRetry(): boolean {
    const last = messages().at(-1);
    return !streaming() && (last?.role === 'user' || last?.status === 'error');
  }

  function dispose(): void {
    disposed = true;
    if (requestId !== null) void ipc.chatCancel(requestId).catch(() => undefined);
    detach?.();
    stopAnimation?.();
    window.clearTimeout(restTimer);
    clearAgentState(options.panelId);
  }

  return {
    conversation,
    setConversation,
    messages,
    streaming,
    error,
    load,
    send,
    stop,
    retry: () => send(null, []),
    canRetry,
    dispose,
  };
}

export type ChatSession = ReturnType<typeof createChatSession>;

function byTime(a: ChatMessage, b: ChatMessage): number {
  return a.at - b.at;
}
