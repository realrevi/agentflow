// The conversation as a timeline (agent/timeline.ts): prompts, replies (sanitized markdown),
// reasoning, tool cards, grouped reads and turn ends, then the live part (streaming reply,
// live reasoning, the permission card) and notices. Code copy buttons and links are
// handled by delegation: http(s) links open in the system browser, right-click offers
// "copy link"; nothing ever navigates inside the app.
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  onCleanup,
  onMount,
  Show,
  Switch,
} from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { ChatError, ChatMessage, Compaction } from '../../shared/types';
import { Menu } from '../../shared/ui/Menu';
import { webUrl } from '../../shared/url';
import { reportError, t } from '../i18n';
import { AgentItem } from './agent/AgentItem';
import type { AgentView } from './agent/agentView';
import { LiveSection } from './agent/LiveSection';
import { NoticeStrip } from './agent/NoticeStrip';
import { buildTimeline, reconcile, type TimelineItem } from './agent/timeline';
import { CompactDivider, ReplyMessage, StreamingReply, UserMessage } from './MessageItem';
import { errorText } from './messageText';
import type { Streaming } from './session';
import { Button } from '../../shared/ui/Button';

interface MessageListProps {
  messages: readonly ChatMessage[];
  streaming: Streaming | null;
  error: ChatError | null;
  canRetry: boolean;
  onRetry: () => void;
  /** Older messages a Compact summarized (requests send the summary instead). */
  compaction: Compaction | null;
  agent: AgentView;
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

  const items = createMemo<TimelineItem[]>(
    (previous) =>
      reconcile(
        previous,
        buildTimeline(props.messages, props.compaction, props.agent.streamingTurn()),
      ),
    [],
  );
  const live = () => props.agent.session.live;

  const follow = () => {
    if (list && pinned) list.scrollTop = list.scrollHeight;
  };
  createEffect(() => {
    void props.messages.length;
    void props.streaming?.text;
    void items().length;
    void live().reasoning()?.text;
    void live().permission();
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
      <For each={items()}>
        {(item) => (
          <Switch fallback={<AgentItem item={item} view={props.agent} />}>
            <Match when={item.kind === 'user' && item}>
              {(user) => <UserMessage message={user().message} latestCopy={latestCopy()} />}
            </Match>
            <Match when={item.kind === 'reply' && item}>
              {(reply) => <ReplyMessage message={reply().message} />}
            </Match>
            <Match when={item.kind === 'compaction' && item}>
              {(divider) => <CompactDivider compaction={divider().compaction} />}
            </Match>
          </Switch>
        )}
      </For>
      <Show when={props.streaming}>
        {(streaming) => (
          <LiveSection
            view={props.agent}
            reply={(busy) => (
              <Show when={streaming().text || !busy}>
                <StreamingReply text={streaming().text} />
              </Show>
            )}
          />
        )}
      </Show>
      <NoticeStrip
        notices={live().notices()}
        canContinue={props.agent.canContinue()}
        onContinue={props.agent.onContinue}
      />
      <Show when={props.error}>{(error) => <p class="af-chat-error">{errorText(error())}</p>}</Show>
      <Show when={props.canRetry}>
        <Button class="af-chat-retry" variant="secondary" size="sm" onClick={() => props.onRetry()}>
          {t('chat.retry')}
        </Button>
      </Show>
    </div>
  );
}
