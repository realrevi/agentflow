// Lazy-loaded dock panel for one API chat (see layout/renderers.tsx). Default export.
// Header (provider, model, system prompt), the messages, and the shared prompt bar with the
// panel's character; files dragged from the tree onto the bottom area are attached. Phase 6:
// the live estimate, the context strip with Compact (or auto-compact), the hard budget stop
// with "Allow once", and the session usage on the corner badge. Phase 7: agent turns (tool
// cards, permission cards with diffs, undo), the running tool as the character's bubble;
// file hooks (open, dirty check, tree marks, the diff view) come from the layout.
import './chat.css';
import './agent.css';
import './permission.css';
import type { DockviewPanelApi } from 'dockview-core';
import { createEffect, createSignal, onCleanup, onMount, Show } from 'solid-js';
import { FILE_PATH_MIME } from '../../shared/dnd';
import type { Attachment } from '../../shared/types';
import { agentState, noteUserInput } from '../agentui';
import {
  AgentOverlay,
  createPromptHistory,
  playPacket,
  PromptBar,
  type AgentOverlayHandle,
  type PromptBarHandle,
  type SentPrompt,
} from '../agentui/components';
import { reducedMotion } from '../characters';
import { reportError, t } from '../i18n';
import { providerName } from '../providers';
import { settings } from '../settings/settingsStore';
import { createAgentView, type ChatHooks } from './agent/agentView';
import { chatLinkHooks, linkChat } from './agentLink';
import { AttachmentChips } from './AttachmentChips';
import { BudgetNotice, ContextStrip, EstimateLine } from './ComposeNotices';
import { readAttachment, withAttachment } from './attachments';
import { ChatHeader } from './ChatHeader';
import { chatTitle, parseChatConfig, type ChatConfig } from './config';
import { listedTitle, refreshConversations } from './conversationsStore';
import { MessageList } from './MessageList';
import { createChatUsage } from './chatUsage';
import { createChatSession } from './session';
import { Button } from '../../shared/ui/Button';

export interface ChatPanelProps {
  panelId: string;
  config: ChatConfig;
  api: DockviewPanelApi;
  hooks: ChatHooks;
}

const PACKET_MS = 600;

export default function ChatPanel(props: ChatPanelProps) {
  const [config, setConfig] = createSignal(props.config);
  const [files, setFiles] = createSignal<readonly Attachment[]>([]);
  const [dropping, setDropping] = createSignal(false);
  const [lastPrompt, setLastPrompt] = createSignal<{ text: string; at: number } | null>(null);
  const [draft, setDraft] = createSignal('');
  let root: HTMLDivElement | undefined;
  let overlay: AgentOverlayHandle | undefined;
  let bar: PromptBarHandle | undefined;

  const setTitle = (title: string) => {
    props.api.setTitle(chatTitle({ title }));
    props.hooks.setTitle(props.panelId, title);
  };
  const session = createChatSession({
    panelId: props.panelId,
    workspace: props.config.workspace,
    conversationId: props.config.conversationId,
    onTitle: setTitle,
    onChanged: () => void refreshConversations(),
    isVisible: () => props.api.isVisible,
    focus: () => props.api.setActive(),
    onFilesChanged: (paths) => props.hooks.filesChanged(paths),
    ...chatLinkHooks(
      props.panelId,
      () => aiName(),
      () => session.messages(),
    ),
  });
  const history = createPromptHistory(
    props.config.workspace,
    props.config.historyKey ?? props.panelId,
  );
  const character = () => (settings.characters.enabled ? config().character : null);
  const provider = () => {
    const id = session.conversation()?.providerId;
    return settings.providers.find((p) => p.id === id);
  };
  const ready = () => Boolean(provider() && session.conversation()?.model.trim());
  const aiName = () => {
    const entry = provider();
    return session.conversation()?.model || (entry ? providerName(entry) : t('chat.untitled'));
  };

  linkChat(
    {
      panelId: props.panelId,
      ready,
      deliver: (text) => bar?.deliver(text) ?? send(text),
      label: aiName,
      role: () => session.conversation()?.agent?.profile?.name ?? null,
    },
    session,
  );
  const agentView = createAgentView(session, props.config.workspace, props.hooks, ready);

  const { usage, estimate, nearLimit, sentBefore } = createChatUsage({
    panelId: props.panelId,
    workspace: props.config.workspace,
    conversationId: props.config.conversationId,
    session,
    draft,
    files,
    ready,
  });

  const allowOnce = async () => {
    const blocked = session.budget.blocked();
    if (!blocked) return;
    if (await session.budget.allow()) {
      if (blocked.text) history.add(blocked.text);
    } else if (!session.budget.blocked()) {
      restore(blocked.text, blocked.attachments);
    }
  };
  const cancelBlocked = () => {
    const blocked = session.budget.dismiss();
    if (blocked) restore(blocked.text, blocked.attachments);
  };
  const restore = (text: string | null, attached: readonly Attachment[]) => {
    if (text) bar?.insert(text);
    setFiles(attached);
  };

  // A rename in the sidebar reaches the open panel.
  createEffect(() => {
    const listed = listedTitle(props.config.conversationId);
    if (listed !== undefined) setTitle(listed);
  });

  const send = (text: string) => {
    if (!ready() || session.streaming()) return false;
    const attached = [...files()];
    setFiles([]);
    setLastPrompt({ text, at: Date.now() });
    void session.send(text, attached).then((ok) => {
      if (ok) history.add(text);
      // A hard budget stop keeps the prompt in its notice until "Allow once" or cancel.
      else if (!session.budget.blocked()) restore(text, attached);
    });
    return true;
  };

  const flyPacket = (sent: SentPrompt) => {
    const target = overlay;
    if (!target || !character()) return;
    if (!settings.characters.packet || reducedMotion() || !root) return target.receive();
    playPacket({
      host: root,
      ...sent,
      target: target.target(),
      duration: PACKET_MS / Math.max(0.25, settings.characters.speed),
      onArrive: () => target.receive(),
    });
  };

  const accepts = (event: DragEvent) => event.dataTransfer?.types.includes(FILE_PATH_MIME) ?? false;
  const onDrop = async (event: DragEvent) => {
    setDropping(false);
    const path = event.dataTransfer?.getData(FILE_PATH_MIME);
    if (!accepts(event) || !path) return;
    event.preventDefault();
    try {
      const file = await readAttachment(path);
      setFiles((list) => withAttachment(list, file));
    } catch (err) {
      reportError('error.attach', err);
    }
  };

  onMount(() => {
    void session.load();
    // The panel loads lazily: its parameters (e.g. the assigned character) may have changed
    // before it mounted.
    const current = parseChatConfig(props.api.getParameters<{ config?: unknown }>().config);
    if (current) setConfig(current);
    const subscriptions = [
      props.api.onDidParametersChange((params) => {
        const next = parseChatConfig((params as { config?: unknown }).config);
        if (next) setConfig(next);
      }),
      props.api.onDidActiveChange((event) => event.isActive && bar?.focus()),
    ];
    if (props.api.isActive) queueMicrotask(() => bar?.focus());
    onCleanup(() => {
      for (const subscription of subscriptions) subscription.dispose();
      session.dispose();
    });
  });

  return (
    <div ref={root} class="af-chat">
      <Show when={session.conversation()}>
        {(conversation) => (
          <ChatHeader
            panelId={props.panelId}
            workspace={props.config.workspace}
            conversation={conversation()}
            onChanged={session.setConversation}
            checkpoints={session.agent.checkpoints}
          />
        )}
      </Show>
      <div class="af-chat-body">
        <MessageList
          messages={session.messages()}
          streaming={session.streaming()}
          error={session.error()}
          canRetry={session.canRetry()}
          onRetry={() => void session.retry()}
          compaction={session.conversation()?.compaction ?? null}
          agent={agentView}
        />
        <Show when={character()}>
          {(ref) => (
            <AgentOverlay
              ref={(handle) => (overlay = handle)}
              character={ref()}
              aiName={aiName()}
              state={agentState(props.panelId)}
              lastPrompt={lastPrompt()}
              history={history.entries()}
              onPickPrompt={(text) => bar?.insert(text)}
              usage={usage()}
              status={session.agent.live.status()}
            />
          )}
        </Show>
      </div>
      <div
        class="af-chat-compose"
        classList={{ dropping: dropping() }}
        onDragOver={(event) => {
          if (!accepts(event)) return;
          event.preventDefault();
          if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
          setDropping(true);
        }}
        onDragLeave={() => setDropping(false)}
        onDrop={(event) => void onDrop(event)}
      >
        <Show when={session.budget.blocked()}>
          {(blocked) => (
            <BudgetNotice
              blocked={blocked()}
              allowing={session.budget.allowing()}
              onAllow={() => void allowOnce()}
              onCancel={cancelBlocked}
            />
          )}
        </Show>
        <Show when={nearLimit() && estimate()}>
          {(current) => (
            <ContextStrip
              estimate={current()}
              compacting={session.compacting()}
              onCompact={() => void session.compact()}
            />
          )}
        </Show>
        <AttachmentChips
          files={files()}
          sentBefore={sentBefore}
          onRemove={(path) => setFiles((list) => list.filter((file) => file.path !== path))}
        />
        <div class="af-chat-row">
          <div class="af-chat-bar">
            <PromptBar
              ref={(handle) => (bar = handle)}
              history={history}
              character={character}
              enabled={ready}
              disabledPlaceholder={t('chat.notReady')}
              onSend={send}
              onSent={flyPacket}
              onDraft={setDraft}
              onTyped={() => noteUserInput(props.panelId)}
            />
          </div>
          <Show when={session.streaming()}>
            <Button class="af-chat-stop" variant="danger" size="sm" onClick={() => session.stop()}>
              {t('chat.stop')}
            </Button>
          </Show>
        </div>
        <EstimateLine estimate={estimate()} />
        <Show when={dropping()}>
          <div class="af-chat-drop">{t('chat.attach.drop')}</div>
        </Show>
      </div>
    </div>
  );
}
