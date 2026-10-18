// API chat panels: start a new chat (creates the conversation), reopen one from the
// sidebar (or focus the panel that shows it), and close panels of a deleted chat.
import type { DockviewApi, IDockviewPanel } from 'dockview-core';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { AgentProfile } from '../../shared/types';
import { loadCharacters } from '../characters';
import { createConversation, newChatConfig, type ChatConfig } from '../chat';
import { reportError, t } from '../i18n';
import { settings } from '../settings/settingsStore';
import { addPanel, assignCharacters, currentDock, nextIndex } from './layoutStore';
import { parsePanelParams } from './panels';

function chatPanel(dock: DockviewApi, conversationId: string): IDockviewPanel | undefined {
  return dock.panels.find((panel) => {
    const params = parsePanelParams(panel.params);
    return params?.kind === 'chat' && params.config.conversationId === conversationId;
  });
}

/** Opens a chat panel in the agent area (or focuses the one showing this conversation).
 * Returns the panel id. */
export function openChat(config: ChatConfig, groupId?: string): string | null {
  const dock = currentDock();
  if (!dock) return null;
  const existing = chatPanel(dock, config.conversationId);
  if (existing) {
    existing.api.setActive();
    return existing.id;
  }
  const index = nextIndex(dock, 'chat');
  addPanel(dock, { kind: 'chat', index, config }, groupId);
  assignCharacters();
  return `chat-${index}`;
}

/** "API agent…": a new conversation with this provider and model, in a new panel. */
export async function startChat(
  providerId: string,
  model: string,
  groupId?: string,
): Promise<string | null> {
  const conversation = await createConversation(providerId, model);
  if (!conversation) return null;
  return openChat(newChatConfig(conversation.id, settings.currentWorkspace), groupId);
}

/**
 * "New API agent → <profile>": a chat with an installed AgentFlow agent. The provider is the
 * profile's hint (provider id, preset or kind) when configured, else the first one.
 */
export async function openProfileChat(
  profile: AgentProfile,
  groupId?: string,
): Promise<string | null> {
  const hint = profile.provider;
  const entry =
    settings.providers.find(
      (p) => hint && (p.id === hint || p.preset === hint || p.kind === hint),
    ) ?? settings.providers[0];
  const model = profile.model ?? entry?.defaultModel ?? '';
  if (!entry || !model.trim()) {
    showToast({ kind: 'error', title: t('installer.profile.noProvider') });
    return null;
  }
  try {
    const workspace = settings.currentWorkspace;
    const conversation = await ipc.agentsProfileChat(workspace, profile.id, entry.id, model);
    const sprite = (await loadCharacters()).find((sheet) => sheet.id === profile.character);
    return openChat(
      {
        ...newChatConfig(conversation.id, workspace),
        title: conversation.title,
        character: sprite ? { id: sprite.id, palette: 0 } : null,
      },
      groupId,
    );
  } catch (err) {
    reportError('error.installerProfile', err);
    return null;
  }
}

/** Reopens a conversation of the current workspace from the sidebar. */
export function reopenChat(conversationId: string, title: string): void {
  openChat({ ...newChatConfig(conversationId, settings.currentWorkspace), title });
}

/** A chat named itself: only `title` changes; the rest of the config (e.g. the character)
 * comes from the panel's current parameters, never from a copy. */
export function setChatTitle(panelId: string, title: string): void {
  const panel = currentDock()?.getPanel(panelId);
  const params = panel ? parsePanelParams(panel.params) : null;
  if (!panel || params?.kind !== 'chat' || params.config.title === title) return;
  panel.api.updateParameters({ ...params, config: { ...params.config, title } });
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
