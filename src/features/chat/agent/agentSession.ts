// The agent side of a chat session: the live turn, checkpoints, the "waiting for your
// approval" toast when the panel is not visible, and the tree marks for files the agent
// wrote (refreshed when a step with writes is stored).
import { showToast } from '../../../shared/toast';
import type { ChatMessage, PermissionRequest } from '../../../shared/types';
import type { AgentState } from '../../characters';
import { t } from '../../i18n';
import { createCheckpoints } from './checkpoints';
import { createLiveTurn } from './liveTurn';
import { activeText } from './toolText';

const WRITE_TOOLS: ReadonlySet<string> = new Set(['write_file', 'edit_file']);

export interface AgentSessionOptions {
  workspace: string | null;
  conversationId: string;
  messages: () => readonly ChatMessage[];
  setState: (state: AgentState) => void;
  /** Whether the panel can be seen (else a permission request also toasts). */
  isVisible: () => boolean;
  title: () => string;
  focus: () => void;
  /** Files an agent wrote, edited or restored (the file tree marks them). */
  onFilesChanged: (paths: readonly string[]) => void;
}

/** A stored `tool` message with at least one successful write or edit. */
export function wroteFiles(toolMessage: ChatMessage, messages: readonly ChatMessage[]): boolean {
  const writes = new Set<string>();
  for (const message of messages) {
    for (const part of message.parts ?? []) {
      if (part.type === 'toolCall' && WRITE_TOOLS.has(part.name)) writes.add(part.id);
    }
  }
  return (toolMessage.parts ?? []).some(
    (part) =>
      part.type === 'toolResult' &&
      writes.has(part.callId) &&
      !part.isError &&
      (part.outcome ?? 'ran') === 'ran',
  );
}

export function createAgentSession(options: AgentSessionOptions) {
  const checkpoints = createCheckpoints(
    options.workspace,
    options.conversationId,
    options.onFilesChanged,
  );
  const asked = (request: PermissionRequest) => {
    if (options.isVisible()) return;
    showToast({
      kind: 'info',
      title: t('agent.toast.waiting', { name: options.title() }),
      detail: activeText({ id: request.callId, name: request.tool, input: request.input }),
      action: { label: t('agent.toast.show'), run: options.focus },
    });
  };
  const live = createLiveTurn(options.conversationId, { phase: options.setState, asked });

  const currentTurn = () =>
    [...options.messages()].reverse().find((message) => message.role === 'user')?.id ?? null;

  /** A step was stored (`message` event). */
  async function stepStored(message: ChatMessage): Promise<void> {
    live.stepStored(message.role);
    if (message.role !== 'tool' || !wroteFiles(message, options.messages())) return;
    await checkpoints.refresh();
    const turn = currentTurn();
    const files = turn ? checkpoints.byTurn(turn)?.files : undefined;
    if (files) options.onFilesChanged(files);
  }

  return { live, checkpoints, stepStored, currentTurn };
}

export type AgentSession = ReturnType<typeof createAgentSession>;
