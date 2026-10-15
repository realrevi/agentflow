// The current workspace's conversations, for the Agents sidebar. Refreshed when the
// workspace changes and after anything that changes a conversation (no polling).
import { createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { Conversation, ConversationSummary } from '../../shared/types';
import { reportError } from '../i18n';
import { settings } from '../settings/settingsStore';

const [list, setList] = createSignal<readonly ConversationSummary[]>([]);
let generation = 0;

/** Reactive, most recently updated first. */
export const conversations = list;

export async function refreshConversations(): Promise<void> {
  const run = ++generation;
  const workspace = settings.currentWorkspace;
  try {
    const next = await ipc.conversationsList(workspace);
    if (run === generation) setList(next);
  } catch (err) {
    reportError('error.conversations', err);
  }
}

export async function createConversation(
  providerId: string | null,
  model: string,
): Promise<Conversation | null> {
  try {
    const conversation = await ipc.conversationCreate(settings.currentWorkspace, providerId, model);
    void refreshConversations();
    return conversation;
  } catch (err) {
    reportError('error.conversationUpdate', err);
    return null;
  }
}

export async function renameConversation(id: string, title: string): Promise<boolean> {
  try {
    await ipc.conversationUpdate(settings.currentWorkspace, id, { title });
    await refreshConversations();
    return true;
  } catch (err) {
    reportError('error.conversationUpdate', err);
    return false;
  }
}

export async function deleteConversation(id: string): Promise<boolean> {
  try {
    await ipc.conversationDelete(settings.currentWorkspace, id);
    await refreshConversations();
    return true;
  } catch (err) {
    reportError('error.conversationDelete', err);
    return false;
  }
}

/** The conversation's current title as listed (for panels to follow renames). */
export function listedTitle(id: string): string | undefined {
  return list().find((item) => item.id === id)?.title;
}
