// Provider + model pickers and the (collapsible) system prompt of one conversation.
// Model ids come from the provider's model list or can be typed by hand. The agent's
// permission mode (ModePill) and the checkpoints menu sit at the end.
import { createSignal, For, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { Conversation, ConversationPatch, PermissionMode } from '../../shared/types';
import { IconButton } from '../../shared/ui/Button';
import { reportError, t } from '../i18n';
import { loadModels, modelsOf, providerName } from '../providers';
import { settings } from '../settings/settingsStore';
import { CheckpointMenu } from './agent/CheckpointMenu';
import type { Checkpoints } from './agent/checkpoints';
import { ModePill } from './agent/ModePill';
import { Input, Select, Textarea } from '../../shared/ui/Field';

interface ChatHeaderProps {
  panelId: string;
  workspace: string | null;
  conversation: Conversation;
  onChanged: (conversation: Conversation) => void;
  checkpoints: Checkpoints;
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

  const changeMode = async (mode: PermissionMode) => {
    const before = props.conversation;
    props.onChanged({ ...before, agent: { ...before.agent, permissionMode: mode } });
    try {
      await ipc.conversationUpdate(props.workspace, before.id, { permissionMode: mode });
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
        <Select
          class="af-chat-provider"
          id={id('provider')}
          value={providerId() ?? ''}
          onChange={(event) => changeProvider(event.currentTarget.value)}
        >
          <option value="">{t('chat.noProvider')}</option>
          <For each={settings.providers}>
            {(entry) => <option value={entry.id}>{providerName(entry)}</option>}
          </For>
        </Select>
        <label class="af-visually-hidden" for={id('model')}>
          {t('chat.model')}
        </label>
        <Input
          class="af-chat-model"
          id={id('model')}
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
        <IconButton
          icon="refresh-cw"
          label={t('chat.models.refresh')}
          disabled={!providerId() || modelsOf(providerId() ?? '').loading}
          onClick={() => {
            const current = providerId();
            if (current) void loadModels(current, true);
          }}
        />
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
        <ModePill
          mode={props.conversation.agent.permissionMode ?? settings.agent.defaultMode}
          onChange={(mode) => void changeMode(mode)}
        />
        <CheckpointMenu checkpoints={props.checkpoints} disabled={!props.workspace} />
      </div>
      <Show when={modelsOf(providerId() ?? '').error}>
        {(error) => <p class="af-chat-header-error">{error()}</p>}
      </Show>
      <Show when={systemOpen()}>
        <Textarea
          class="af-chat-system"
          id={id('system')}
          rows={3}
          placeholder={t('chat.system.placeholder')}
          value={props.conversation.systemPrompt}
          onChange={(event) => void save({ systemPrompt: event.currentTarget.value })}
        />
      </Show>
    </header>
  );
}
