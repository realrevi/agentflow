// The conversation: user prompts (plain text + attachment chips) and replies (sanitized
// markdown). The streaming reply re-renders only its last block. Code copy buttons and
// links are handled by delegation: http(s) links open in the system browser, right-click
// offers "copy link"; nothing ever navigates inside the app. Agent turns add reasoning, tool
// lines and the permission row (AgentParts.tsx, PermissionRow.tsx); `tool` messages render as
// the results under their calls.
import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { ChatError, ChatMessage, Compaction, ToolResult } from '../../shared/types';
import { Menu } from '../../shared/ui/Menu';
import { webUrl } from '../../shared/url';
import { reportError, t } from '../i18n';
import { MessageCalls, Notices, Reasoning, reasoningOf, resultsOf } from './AgentParts';
import type { LiveTool } from './agentTurn';
import type { AgentTurn } from './agentTurn';
import { estimateTokens } from './attachments';
import { renderMarkdown } from './markdown';
import { cacheText, errorText, stopNote, usageText } from './messageText';
import { PermissionRow } from './PermissionRow';
import type { Streaming } from './session';
import { splitStable } from './stableSplit';

interface MessageListProps {
  messages: readonly ChatMessage[];
  streaming: Streaming | null;
  error: ChatError | null;
  canRetry: boolean;
  onRetry: () => void;
  /** Older messages a Compact summarized (requests send the summary instead). */
  compaction: Compaction | null;
  agent: AgentTurn;
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

  // For each attached path, the message that sent its latest copy (older copies are only
  // referenced in requests: the backend sends a file once).
  const latestCopy = createMemo(() => {
    const latest = new Map<string, string>();
    for (const message of props.messages) {
      for (const file of message.attachments) latest.set(file.path, message.id);
    }
    return latest;
  });

  const results = createMemo(() => resultsOf(props.messages));
  /** A tool runs, a permission waits or reasoning streams: no empty reply caret. */
  const busy = () =>
    Boolean(props.agent.permission() || props.agent.reasoning()) ||
    props.agent.tools().some((tool) => tool.ok === null);

  const follow = () => {
    if (list && pinned) list.scrollTop = list.scrollHeight;
  };
  createEffect(() => {
    void props.messages.length;
    void props.streaming?.text;
    void props.agent.tools().length;
    void props.agent.reasoning();
    void props.agent.permission();
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
      <For each={props.messages}>
        {(message) => (
          <>
            <Show when={message.role !== 'tool'}>
              <MessageItem
                message={message}
                latestCopy={latestCopy()}
                results={results()}
                live={props.agent.tools()}
              />
            </Show>
            <Show when={props.compaction?.through === message.id && props.compaction}>
              {(compaction) => <CompactDivider compaction={compaction()} />}
            </Show>
          </>
        )}
      </For>
      <Show when={props.streaming}>
        {(streaming) => (
          <>
            <Show when={props.agent.reasoning()}>{(text) => <Reasoning text={text()} live />}</Show>
            <Show when={streaming().text || !busy()}>
              <StreamingReply text={streaming().text} />
            </Show>
            <Show when={props.agent.permission()}>
              {(request) => (
                <PermissionRow request={request()} onAnswer={(a) => void props.agent.answer(a)} />
              )}
            </Show>
          </>
        )}
      </Show>
      <Notices notices={props.agent.notices()} />
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

function CompactDivider(props: { compaction: Compaction }) {
  return (
    <details class="af-chat-compacted">
      <summary>{t('chat.compact.divider')}</summary>
      <p class="af-chat-compacted-summary">{props.compaction.summary}</p>
      <p class="af-chat-compacted-by">{t('chat.compact.by', { model: props.compaction.model })}</p>
    </details>
  );
}

function MessageItem(props: {
  message: ChatMessage;
  latestCopy: Map<string, string>;
  results: Map<string, ToolResult>;
  live: readonly LiveTool[];
}) {
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
                  {(file) => {
                    const replaced = () => props.latestCopy.get(file.path) !== props.message.id;
                    return (
                      <span
                        class="af-chip"
                        classList={{ 'sent-earlier': replaced() }}
                        title={
                          replaced() ? t('chat.attach.replaced', { path: file.path }) : file.path
                        }
                      >
                        {file.path.split('/').pop()}
                        <span class="af-chip-tokens">
                          {replaced()
                            ? t('chat.attach.sentEarlier')
                            : t('chat.attach.tokens', { count: estimateTokens(file.content) })}
                        </span>
                      </span>
                    );
                  }}
                </For>
              </div>
            </Show>
          </>
        }
      >
        <Show when={reasoningOf(props.message)}>{(text) => <Reasoning text={text()} />}</Show>
        <Show when={props.message.content.trim()}>
          <div class="af-md" innerHTML={html()} />
        </Show>
        <MessageCalls message={props.message} results={props.results} live={props.live} />
        <div class="af-chat-meta">
          <Show when={stopNote(props.message)}>{(note) => <span>{note()}</span>}</Show>
          <Show when={props.message.status === 'error' && props.message.error}>
            {(error) => <span class="af-chat-meta-error">{error()}</span>}
          </Show>
          <Show when={usageText(props.message)}>{(usage) => <span>{usage()}</span>}</Show>
          <Show when={cacheText(props.message)}>
            {(cache) => (
              <span class="af-chat-cache" classList={{ hit: cache().hit }}>
                {cache().text}
              </span>
            )}
          </Show>
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
