// What an API chat panel shows: one stored conversation. Persisted in the layout (never the
// messages), so it is parsed defensively on restore.
import { isCharacterRef } from '../characters/catalog';
import type { CharacterRef } from '../characters/types';
import { t } from '../i18n';

export interface ChatConfig {
  conversationId: string;
  /** The workspace the conversation is stored under (`null` = none). */
  workspace: string | null;
  /** Cached for the tab title until the conversation loads. */
  title: string;
  character: CharacterRef | null;
  /** Prompt-bar history key. */
  historyKey: string | null;
}

const ID = /^[A-Za-z0-9_-]{1,64}$/;
const TITLE_MAX = 40;

export function parseChatConfig(raw: unknown): ChatConfig | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const { conversationId, workspace, title, character, historyKey } = raw as Record<
    string,
    unknown
  >;
  if (typeof conversationId !== 'string' || !ID.test(conversationId)) return null;
  return {
    conversationId,
    workspace: typeof workspace === 'string' && workspace.trim() ? workspace : null,
    title: typeof title === 'string' ? title.slice(0, 200) : '',
    character: isCharacterRef(character) ? { id: character.id, palette: character.palette } : null,
    historyKey: typeof historyKey === 'string' && ID.test(historyKey) ? historyKey : null,
  };
}

export function newChatConfig(conversationId: string, workspace: string | null): ChatConfig {
  return {
    conversationId,
    workspace,
    title: '',
    character: null,
    historyKey: crypto.randomUUID(),
  };
}

/** Tab title: the conversation's title, shortened, or "New chat". */
export function chatTitle(config: Pick<ChatConfig, 'title'>): string {
  const title = config.title.trim();
  if (!title) return t('chat.untitled');
  return title.length > TITLE_MAX ? `${title.slice(0, TITLE_MAX - 1)}…` : title;
}
