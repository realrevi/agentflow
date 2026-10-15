// API chat panels: start a new chat (creates the conversation), reopen one from the
// sidebar (or focus the panel that shows it), and close panels of a deleted chat.
import type { DockviewApi, IDockviewPanel } from 'dockview-core';
import { createConversation, newChatConfig, type ChatConfig } from '../chat';
import { settings } from '../settings/settingsStore';
import { addPanel, assignCharacters, currentDock, nextIndex } from './layoutStore';
import { parsePanelParams } from './panels';

function chatPanel(dock: DockviewApi, conversationId: string): IDockviewPanel | undefined {
  return dock.panels.find((panel) => {
    const params = parsePanelParams(panel.params);
    return params?.kind === 'chat' && params.config.conversationId === conversationId;
  });
}

/** Opens a chat panel in the agent area (or focuses the one showing this conversation). */
export function openChat(config: ChatConfig, groupId?: string): void {
  const dock = currentDock();
  if (!dock) return;
  const existing = chatPanel(dock, config.conversationId);
  if (existing) {
    existing.api.setActive();
    return;
  }
  addPanel(dock, { kind: 'chat', index: nextIndex(dock, 'chat'), config }, groupId);
  assignCharacters();
}

/** "API agent…": a new conversation with this provider and model, in a new panel. */
export async function startChat(
  providerId: string,
  model: string,
  groupId?: string,
): Promise<void> {
  const conversation = await createConversation(providerId, model);
  if (!conversation) return;
  openChat(newChatConfig(conversation.id, settings.currentWorkspace), groupId);
}

/** Reopens a conversation of the current workspace from the sidebar. */
export function reopenChat(conversationId: string, title: string): void {
  openChat({ ...newChatConfig(conversationId, settings.currentWorkspace), title });
}

export function closeChatPanels(conversationId: string): void {
  const dock = currentDock();
  if (!dock) return;
  for (const panel of dock.panels) {
    const params = parsePanelParams(panel.params);
    if (params?.kind === 'chat' && params.config.conversationId === conversationId)
      panel.api.close();
  }
}
