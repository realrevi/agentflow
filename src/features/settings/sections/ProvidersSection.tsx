// AI providers for API chat panels. Keys live in the OS keychain (ProviderKeyField); the
// rest of an entry is ordinary settings.
import { createSignal, For, Show } from 'solid-js';
import { askChoice } from '../../../shared/dialog';
import { Menu, type MenuItem } from '../../../shared/ui/Menu';
import type { ProviderPreset } from '../../../shared/types';
import { reportError, t } from '../../i18n';
import {
  forgetModels,
  keyStatusOf,
  newProviderEntry,
  presetLabel,
  PRESETS,
  providerName,
  removeKey,
} from '../../providers';
import { settings, updateSettings } from '../settingsStore';
import { ProviderEditor } from './ProviderEditor';
import { Row, Section } from './Section';

export function ProvidersSection() {
  let addButton: HTMLButtonElement | undefined;
  const [adding, setAdding] = createSignal(false);
  const [open, setOpen] = createSignal<string | null>(null);

  const presetItems = (): MenuItem[] =>
    PRESETS.map((preset, index) => ({
      id: preset.id,
      label: presetLabel(preset.id),
      heading:
        index === 0
          ? t('settings.providers.group.anthropic')
          : index === 1
            ? t('settings.providers.group.openai')
            : undefined,
    }));

  const add = (preset: string) => {
    const entry = newProviderEntry(preset as ProviderPreset);
    updateSettings((s) => s.providers.push(entry));
    setOpen(entry.id);
  };

  const remove = async (id: string) => {
    const entry = settings.providers.find((p) => p.id === id);
    if (!entry) return;
    const choice = await askChoice({
      title: t('settings.providers.removeConfirm.title', { name: providerName(entry) }),
      message: t('settings.providers.removeConfirm.body'),
      choices: [
        { id: 'remove', label: t('settings.providers.remove'), kind: 'danger' },
        { id: 'cancel', label: t('dialog.cancel') },
      ],
      cancelId: 'cancel',
    });
    if (choice !== 'remove') return;
    try {
      await removeKey(id);
    } catch (err) {
      // Keep the entry so its key is not left orphaned in the keychain; the user can retry.
      reportError('error.providerKey', err);
      return;
    }
    forgetModels(id);
    updateSettings((s) => (s.providers = s.providers.filter((p) => p.id !== id)));
  };

  return (
    <Section title={t('settings.providers')} hint={t('settings.providers.hint')}>
      <Show
        when={settings.providers.length > 0}
        fallback={<p class="af-settings-hint">{t('settings.providers.empty')}</p>}
      >
        <ul class="af-provider-list">
          <For each={settings.providers}>
            {(entry) => {
              const expanded = () => open() === entry.id;
              return (
                <li class="af-provider" classList={{ open: expanded() }}>
                  <div class="af-provider-head">
                    <button
                      type="button"
                      class="af-provider-toggle"
                      aria-expanded={expanded()}
                      onClick={() => setOpen(expanded() ? null : entry.id)}
                    >
                      <span class="af-provider-name">{providerName(entry)}</span>
                      <span class="af-provider-meta">
                        {presetLabel(entry.preset)}
                        {entry.defaultModel ? ` · ${entry.defaultModel}` : ''}
                      </span>
                      <span
                        class="af-key-dot"
                        classList={{ ok: keyStatusOf(entry.id)?.hasKey === true }}
                        aria-hidden="true"
                      />
                    </button>
                    <button
                      type="button"
                      class="af-button secondary small"
                      onClick={() => void remove(entry.id)}
                    >
                      {t('settings.providers.remove')}
                    </button>
                  </div>
                  <Show when={expanded()}>
                    <ProviderEditor entry={entry} />
                  </Show>
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
      <button
        ref={addButton}
        type="button"
        class="af-button small"
        aria-haspopup="menu"
        aria-expanded={adding()}
        onClick={() => setAdding((value) => !value)}
      >
        {t('settings.providers.add')}
      </button>
      <Show when={adding() && addButton}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            items={presetItems()}
            placement="below-start"
            onSelect={add}
            onClose={() => setAdding(false)}
          />
        )}
      </Show>
      <Row label={t('settings.chat.smoothReveal')} for="af-chat-reveal">
        <input
          id="af-chat-reveal"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={settings.chat.smoothReveal}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.chat.smoothReveal = on));
          }}
        />
      </Row>
    </Section>
  );
}
