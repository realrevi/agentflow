// "API agent…": pick a provider and a model, then the layout opens a chat panel. With no
// providers yet it points to Settings instead.
import './newChat.css';
import { createSignal, For, onMount, Show } from 'solid-js';
import type { MenuPlacement } from '../../shared/ui/Menu';
import { Popover } from '../../shared/ui/Popover';
import { t } from '../i18n';
import { loadModels, modelsOf, providerName } from '../providers';
import { settings } from '../settings/settingsStore';
import { Button } from '../../shared/ui/Button';
import { Input, Select } from '../../shared/ui/Field';

interface NewChatPromptProps {
  anchor: HTMLElement;
  placement: MenuPlacement;
  onStart: (providerId: string, model: string) => void;
  onOpenSettings: () => void;
  onClose: () => void;
}

let lastProvider: string | null = null;

export function NewChatPrompt(props: NewChatPromptProps) {
  const first = () =>
    settings.providers.find((p) => p.id === lastProvider) ?? settings.providers[0] ?? null;
  const [providerId, setProviderId] = createSignal(first()?.id ?? '');
  const [model, setModel] = createSignal(first()?.defaultModel ?? '');
  const provider = () => settings.providers.find((p) => p.id === providerId());

  onMount(() => {
    if (providerId()) void loadModels(providerId());
  });

  const pick = (id: string) => {
    setProviderId(id);
    setModel(settings.providers.find((p) => p.id === id)?.defaultModel ?? '');
    if (id) void loadModels(id);
  };

  const start = (event: SubmitEvent) => {
    event.preventDefault();
    const id = providerId();
    if (!id || !model().trim()) return;
    lastProvider = id;
    props.onStart(id, model().trim());
    props.onClose();
  };

  return (
    <Popover
      anchor={props.anchor}
      placement={props.placement}
      label={t('chat.new.title')}
      onClose={props.onClose}
    >
      <Show
        when={settings.providers.length > 0}
        fallback={
          <div class="af-new-chat">
            <p>{t('chat.new.noProviders')}</p>
            <Button
              size="sm"
              onClick={() => {
                props.onOpenSettings();
                props.onClose();
              }}
            >
              {t('chat.new.openSettings')}
            </Button>
          </div>
        }
      >
        <form class="af-new-chat" onSubmit={start}>
          <div class="af-new-chat-title">{t('chat.new.title')}</div>
          <label>
            {t('chat.provider')}
            <Select value={providerId()} onChange={(event) => pick(event.currentTarget.value)}>
              <For each={settings.providers}>
                {(entry) => <option value={entry.id}>{providerName(entry)}</option>}
              </For>
            </Select>
          </label>
          <label>
            {t('chat.model')}
            <Input
              list="af-new-chat-models"
              spellcheck={false}
              placeholder={t('chat.model.placeholder')}
              value={model()}
              onInput={(event) => setModel(event.currentTarget.value)}
            />
          </label>
          <datalist id="af-new-chat-models">
            <For each={provider() ? modelsOf(providerId()).list : []}>
              {(item) => <option value={item.id}>{item.name ?? ''}</option>}
            </For>
          </datalist>
          <Show when={modelsOf(providerId()).error}>
            {(error) => <p class="af-new-chat-error">{error()}</p>}
          </Show>
          <Button size="sm" type="submit" disabled={!providerId() || !model().trim()}>
            {t('chat.new.start')}
          </Button>
        </form>
      </Show>
    </Popover>
  );
}
