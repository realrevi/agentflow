// The conversation: user prompts (plain text + attachment chips) and replies (sanitized
// markdown). The streaming reply re-renders only its last block. Code copy buttons and
// links are handled by delegation: http(s) links open in the system browser, right-click
// offers "copy link"; nothing ever navigates inside the app.
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { ChatError, ChatMessage } from '../../shared/types';
import { Menu } from '../../shared/ui/Menu';
import { webUrl } from '../../shared/url';
import { reportError, t } from '../i18n';
import { estimateTokens } from './attachments';
import { renderMarkdown } from './markdown';
import { errorText, stopNote, usageText } from './messageText';
import type { Streaming } from './session';
import { splitStable } from './stableSplit';

interface MessageListProps {
  messages: readonly ChatMessage[];
  streaming: Streaming | null;
  error: ChatError | null;
  canRetry: boolean;
  onRetry: () => void;
}

/** Near the bottom: keep following new text. */
const PIN_SLACK_PX = 48;

export function MessageList(props: MessageListProps) {
  let list: HTMLDivElement | undefined;
  let pinned = true;
  const [linkMenu, setLinkMenu] = createSignal<{ x: number; y: number; url: string } | null>(null);

  const copy = async (text: string, toast: string) => {
    try {
      await ipc.writeClipboard(text);
      showToast({ kind: 'success', title: toast });
    } catch (err) {
      reportError('error.clipboard', err);
    }
  };
  const open = (url: string) => {
    ipc.openUrl(url).catch((err: unknown) => reportError('error.openLink', err));
  };

  const onClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    const button = target.closest<HTMLElement>('[data-copy="code"]');
    if (button) {
      const code = button.closest('.af-code')?.querySelector('code')?.textContent ?? '';
      void copy(code, t('chat.copied'));
      return;
    }
    const link = target.closest<HTMLAnchorElement>('a[href]');
    if (link) {
      event.preventDefault();
      const href = link.getAttribute('href') ?? '';
      const url = webUrl(href);
      if (url) open(url);
      else void copy(href, t('chat.linkCopied', { url: href }));
    }
  };
  const onContextMenu = (event: MouseEvent) => {
    const link = (event.target as HTMLElement).closest<HTMLAnchorElement>('a[href]');
    if (!link) return;
    event.preventDefault();
    setLinkMenu({ x: event.clientX, y: event.clientY, url: link.getAttribute('href') ?? '' });
  };

  const follow = () => {
    if (list && pinned) list.scrollTop = list.scrollHeight;
  };
  createEffect(() => {
    void props.messages.length;
    void props.streaming?.text;
    void props.error;
    queueMicrotask(follow);
  });
  onMount(() => {
    follow();
    if (!list) return;
    const observer = new ResizeObserver(follow);
    observer.observe(list);
    onCleanup(() => observer.disconnect());
  });

  return (
    <div
      ref={list}
      class="af-chat-list"
      role="log"
      aria-live="polite"
      onScroll={() => {
        if (list) pinned = list.scrollHeight - list.scrollTop - list.clientHeight < PIN_SLACK_PX;
      }}
      onClick={onClick}
      onContextMenu={onContextMenu}
    >
      <Show when={linkMenu()}>
        {(menu) => (
          <Menu
            anchor={{ x: menu().x, y: menu().y }}
            placement="below-start"
            items={[
              { id: 'open', label: t('chat.link.open'), disabled: !webUrl(menu().url) },
              { id: 'copy', label: t('chat.link.copy') },
            ]}
            onSelect={(id) => {
              const url = menu().url;
              if (id === 'open') open(url);
              else void copy(url, t('chat.linkCopied', { url }));
            }}
            onClose={() => setLinkMenu(null)}
          />
        )}
      </Show>
      <Show when={props.messages.length === 0 && !props.streaming}>
        <p class="af-chat-empty">{t('chat.empty')}</p>
      </Show>
      <For each={props.messages}>{(message) => <MessageItem message={message} />}</For>
      <Show when={props.streaming}>
        {(streaming) => <StreamingReply text={streaming().text} />}
      </Show>
      <Show when={props.error}>{(error) => <p class="af-chat-error">{errorText(error())}</p>}</Show>
      <Show when={props.canRetry}>
        <button
          type="button"
          class="af-button secondary small af-chat-retry"
          onClick={() => props.onRetry()}
        >
          {t('chat.retry')}
        </button>
      </Show>
    </div>
  );
}

function MessageItem(props: { message: ChatMessage }) {
  const html = createMemo(() =>
    props.message.role === 'assistant' ? renderMarkdown(props.message.content) : '',
  );
  return (
    <article class={`af-chat-message ${props.message.role}`}>
      <Show
        when={props.message.role === 'assistant'}
        fallback={
          <>
            <div class="af-chat-who">{t('chat.you')}</div>
            <div class="af-chat-user-text">{props.message.content}</div>
            <Show when={props.message.attachments.length > 0}>
              <div class="af-chips">
                <For each={props.message.attachments}>
                  {(file) => (
                    <span class="af-chip" title={file.path}>
                      {file.path.split('/').pop()}
                      <span class="af-chip-tokens">
                        {t('chat.attach.tokens', { count: estimateTokens(file.content) })}
                      </span>
                    </span>
                  )}
                </For>
              </div>
            </Show>
          </>
        }
      >
        <div class="af-md" innerHTML={html()} />
        <div class="af-chat-meta">
          <Show when={stopNote(props.message)}>{(note) => <span>{note()}</span>}</Show>
          <Show when={props.message.status === 'error' && props.message.error}>
            {(error) => <span class="af-chat-meta-error">{error()}</span>}
          </Show>
          <Show when={usageText(props.message)}>{(usage) => <span>{usage()}</span>}</Show>
        </div>
      </Show>
    </article>
  );
}

/** Finished blocks are rendered once; only the block being written re-renders per frame. */
function StreamingReply(props: { text: string }) {
  const split = createMemo(() => splitStable(props.text));
  const head = createMemo(() => renderMarkdown(split().head));
  const tail = createMemo(() => renderMarkdown(split().tail));
  return (
    <article class="af-chat-message assistant streaming">
      <div class="af-md" innerHTML={head()} />
      <div class="af-md af-md-tail" innerHTML={tail()} />
      <span class="af-chat-caret" aria-hidden="true" />
    </article>
  );
}
