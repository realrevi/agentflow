// The workspace's API chats under the open panels: click to reopen, "…" to rename or
// delete (moved to the app's trash folder), and a button to start a new one.
import { createEffect, createSignal, For, on, Show } from 'solid-js';
import {
  conversations,
  deleteConversation,
  NewChatPrompt,
  refreshConversations,
  renameConversation,
} from '../../features/chat';
import { t } from '../../features/i18n';
import { closeChatPanels, openPanel, reopenChat, startChat } from '../../features/layout';
import { settings } from '../../features/settings';
import { askChoice } from '../../shared/dialog';
import type { ConversationSummary } from '../../shared/types';
import { InlinePrompt } from '../../shared/ui/InlinePrompt';
import { Menu } from '../../shared/ui/Menu';

type Action = { kind: 'menu' | 'rename'; id: string; anchor: HTMLElement };

export function ChatList() {
  const [action, setAction] = createSignal<Action | null>(null);
  const [asking, setAsking] = createSignal(false);
  let newButton: HTMLButtonElement | undefined;
  createEffect(
    on(
      () => settings.currentWorkspace,
      () => void refreshConversations(),
    ),
  );

  const title = (chat: ConversationSummary) => chat.title || t('chat.untitled');

  const remove = async (chat: ConversationSummary) => {
    const choice = await askChoice({
      title: t('chat.delete.title', { title: title(chat) }),
      message: t('chat.delete.body'),
      choices: [
        { id: 'delete', label: t('chat.delete'), kind: 'danger' },
        { id: 'cancel', label: t('dialog.cancel') },
      ],
      cancelId: 'cancel',
    });
    if (choice === 'delete' && (await deleteConversation(chat.id))) closeChatPanels(chat.id);
  };

  const find = (id: string) => conversations().find((chat) => chat.id === id);

  return (
    <>
      <h2 class="af-view-heading">{t('sidebar.agents.chats')}</h2>
      <Show
        when={conversations().length > 0}
        fallback={<p class="af-view-empty">{t('sidebar.agents.noChats')}</p>}
      >
        <ul class="af-list">
          <For each={conversations()}>
            {(chat) => (
              <li class="af-chat-item">
                <button
                  type="button"
                  class="af-list-item"
                  title={chat.model}
                  onClick={() => reopenChat(chat.id, chat.title)}
                >
                  <span class="af-dot af-dot-chat" aria-hidden="true" />
                  <span class="af-chat-item-title">{title(chat)}</span>
                </button>
                <button
                  type="button"
                  class="af-icon-btn af-chat-item-more"
                  title={t('chat.more', { title: title(chat) })}
                  aria-label={t('chat.more', { title: title(chat) })}
                  aria-haspopup="menu"
                  onClick={(event) =>
                    setAction({ kind: 'menu', id: chat.id, anchor: event.currentTarget })
                  }
                >
                  …
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <button
        ref={newButton}
        type="button"
        class="af-button secondary small af-chat-new"
        aria-haspopup="dialog"
        aria-expanded={asking()}
        onClick={() => setAsking((open) => !open)}
      >
        {t('sidebar.agents.newChat')}
      </button>
      <Show when={asking() && newButton}>
        {(anchor) => (
          <NewChatPrompt
            anchor={anchor()}
            placement="below-start"
            onStart={(providerId, model) => void startChat(providerId, model)}
            onOpenSettings={() => openPanel('settings')}
            onClose={() => setAsking(false)}
          />
        )}
      </Show>
      <Show when={action()?.kind === 'menu' && action()}>
        {(current) => (
          <Menu
            anchor={current().anchor}
            placement="below-end"
            items={[
              { id: 'rename', label: t('chat.rename') },
              { id: 'delete', label: t('chat.delete') },
            ]}
            onSelect={(choice) => {
              const chat = find(current().id);
              if (!chat) return;
              if (choice === 'delete') void remove(chat);
              else queueMicrotask(() => setAction({ ...current(), kind: 'rename' }));
            }}
            onClose={() => setAction((open) => (open?.kind === 'menu' ? null : open))}
          />
        )}
      </Show>
      <Show when={action()?.kind === 'rename' && action()}>
        {(current) => (
          <InlinePrompt
            anchor={current().anchor}
            placement="below-end"
            label={t('chat.rename.prompt')}
            placeholder={find(current().id)?.title ?? ''}
            submitLabel={t('chat.rename')}
            onSubmit={(value) => void renameConversation(current().id, value)}
            onClose={() => setAction(null)}
          />
        )}
      </Show>
    </>
  );
}
