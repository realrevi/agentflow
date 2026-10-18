// The non-agent pieces of a conversation: a user prompt (text + attachment chips), a reply
// (sanitized markdown + usage, cache and stop notes), the streaming reply (only its last
// block re-renders) and the Compact divider.
import { createMemo, For, Show } from 'solid-js';
import { renderMarkdown } from '../../shared/markdown';
import type { ChatMessage, Compaction } from '../../shared/types';
import { t } from '../i18n';
import { estimateTokens } from './attachments';
import { cacheText, stopNote, usageText } from './messageText';
import { splitStable } from './stableSplit';

export function CompactDivider(props: { compaction: Compaction }) {
  return (
    <details class="af-chat-compacted">
      <summary>{t('chat.compact.divider')}</summary>
      <p class="af-chat-compacted-summary">{props.compaction.summary}</p>
      <p class="af-chat-compacted-by">{t('chat.compact.by', { model: props.compaction.model })}</p>
    </details>
  );
}

export function UserMessage(props: { message: ChatMessage; latestCopy: Map<string, string> }) {
  return (
    <article class="af-chat-message user">
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
                  title={replaced() ? t('chat.attach.replaced', { path: file.path }) : file.path}
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
    </article>
  );
}

export function ReplyMessage(props: { message: ChatMessage }) {
  const html = createMemo(() =>
    renderMarkdown(props.message.content, { copyLabel: t('chat.copyCode') }),
  );
  return (
    <article class="af-chat-message assistant">
      <Show when={props.message.content.trim()}>
        <div class="af-md" innerHTML={html()} />
      </Show>
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
    </article>
  );
}

/** Finished blocks are rendered once; only the block being written re-renders per frame. */
export function StreamingReply(props: { text: string }) {
  const split = createMemo(() => splitStable(props.text));
  const head = createMemo(() => renderMarkdown(split().head, { copyLabel: t('chat.copyCode') }));
  const tail = createMemo(() => renderMarkdown(split().tail, { copyLabel: t('chat.copyCode') }));
  return (
    <article class="af-chat-message assistant streaming">
      <div class="af-md" innerHTML={head()} />
      <div class="af-md af-md-tail" innerHTML={tail()} />
      <span class="af-chat-caret" aria-hidden="true" />
    </article>
  );
}
