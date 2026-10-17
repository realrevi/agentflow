// Provider + model pickers and the (collapsible) system prompt of one conversation.
// Model ids come from the provider's model list or can be typed by hand. The agent's
// permission mode and checkpoints sit at the end (AgentControls.tsx).
import { createSignal, For, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { Conversation, ConversationPatch } from '../../shared/types';
import { refreshIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { reportError, t } from '../i18n';
import { loadModels, modelsOf, providerName } from '../providers';
import { settings } from '../settings/settingsStore';
import { AgentControls } from './AgentControls';

interface ChatHeaderProps {
  panelId: string;
  workspace: string | null;
  conversation: Conversation;
  onChanged: (conversation: Conversation) => void;
}

export function ChatHeader(props: ChatHeaderProps) {
  const [systemOpen, setSystemOpen] = createSignal(props.conversation.systemPrompt.length > 0);
  const id = (part: string) => `af-chat-${props.panelId}-${part}`;
  const providerId = () => props.conversation.providerId;

  const save = async (patch: ConversationPatch) => {
    const before = props.conversation;
    props.onChanged({
      ...before,
      ...patch,
      providerId: patch.providerId === undefined ? before.providerId : patch.providerId,
    });
    try {
      await ipc.conversationUpdate(props.workspace, before.id, patch);
    } catch (err) {
      props.onChanged(before);
      reportError('error.conversationUpdate', err);
    }
  };

  const changeProvider = (next: string) => {
    const entry = settings.providers.find((p) => p.id === next);
    const patch: ConversationPatch = { providerId: next || null };
    if (entry && (!props.conversation.model || props.conversation.providerId !== next)) {
      patch.model = entry.defaultModel;
    }
    void save(patch);
    if (next) void loadModels(next);
  };

  return (
    <header class="af-chat-header">
      <div class="af-chat-pickers">
        <label class="af-visually-hidden" for={id('provider')}>
          {t('chat.provider')}
        </label>
        <select
          id={id('provider')}
          class="af-select af-chat-provider"
          value={providerId() ?? ''}
          onChange={(event) => changeProvider(event.currentTarget.value)}
        >
          <option value="">{t('chat.noProvider')}</option>
          <For each={settings.providers}>
            {(entry) => <option value={entry.id}>{providerName(entry)}</option>}
          </For>
        </select>
        <label class="af-visually-hidden" for={id('model')}>
          {t('chat.model')}
        </label>
        <input
          id={id('model')}
          class="af-input af-chat-model"
          list={id('models')}
          spellcheck={false}
          placeholder={t('chat.model.placeholder')}
          value={props.conversation.model}
          onFocus={() => {
            const current = providerId();
            if (current) void loadModels(current);
          }}
          onChange={(event) => void save({ model: event.currentTarget.value.trim() })}
        />
        <datalist id={id('models')}>
          <For each={providerId() ? modelsOf(providerId() ?? '').list : []}>
            {(model) => <option value={model.id}>{model.name ?? ''}</option>}
          </For>
        </datalist>
        <button
          type="button"
          class="af-icon-btn"
          title={t('chat.models.refresh')}
          aria-label={t('chat.models.refresh')}
          disabled={!providerId() || modelsOf(providerId() ?? '').loading}
          onClick={() => {
            const current = providerId();
            if (current) void loadModels(current, true);
          }}
        >
          <PixelIcon art={refreshIcon} />
        </button>
        <button
          type="button"
          class="af-chat-system-toggle"
          aria-expanded={systemOpen()}
          aria-controls={id('system')}
          onClick={() => setSystemOpen((open) => !open)}
        >
          {t('chat.system')}
          <Show when={props.conversation.systemPrompt && !systemOpen()}>
            <span class="af-chat-system-dot" aria-hidden="true" />
          </Show>
        </button>
        <AgentControls
          panelId={props.panelId}
          workspace={props.workspace}
          conversation={props.conversation}
          onChanged={props.onChanged}
        />
      </div>
      <Show when={modelsOf(providerId() ?? '').error}>
        {(error) => <p class="af-chat-header-error">{error()}</p>}
      </Show>
      <Show when={systemOpen()}>
        <textarea
          id={id('system')}
          class="af-input af-chat-system"
          rows={3}
          placeholder={t('chat.system.placeholder')}
          value={props.conversation.systemPrompt}
          onChange={(event) => void save({ systemPrompt: event.currentTarget.value })}
        />
      </Show>
    </header>
  );
}
