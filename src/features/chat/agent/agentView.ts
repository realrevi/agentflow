// What the message list needs for agent turns, built by the chat panel from its session
// and the hooks the layout lends it.
import type { Component } from 'solid-js';
import type { ChatSession } from '../session';
import type { AgentSession } from './agentSession';
import type { DiffViewProps } from './PermissionCard';

/** What the layout lends a chat panel (chat never imports the editor or the file tree). */
export interface ChatHooks {
  openFile: (path: string) => void;
  isFileDirty: (path: string) => boolean;
  filesChanged: (paths: readonly string[]) => void;
  Diff: Component<DiffViewProps>;
}

export interface AgentView {
  session: AgentSession;
  /** The user message id of the turn that is streaming. */
  streamingTurn: () => string | null;
  workspace: string | null;
  isFileDirty: (path: string) => boolean;
  openFile: (path: string) => void;
  Diff: Component<DiffViewProps>;
  /** "Continue" after the step limit. */
  canContinue: () => boolean;
  onContinue: () => void;
}

export function createAgentView(
  session: ChatSession,
  workspace: string | null,
  hooks: ChatHooks,
  ready: () => boolean,
): AgentView {
  return {
    session: session.agent,
    streamingTurn: session.streamingTurn,
    workspace,
    isFileDirty: (path) => hooks.isFileDirty(path),
    openFile: (path) => hooks.openFile(path),
    Diff: hooks.Diff,
    canContinue: () => ready() && !session.streaming(),
    onContinue: () => void session.continueTurn(),
  };
}
