// Write-only API key field: the key goes straight to the OS keychain and is never shown
// again; the form only knows whether one is saved and its last 4 characters.
import { createSignal, onMount, Show } from 'solid-js';
import { showToast } from '../../../shared/toast';
import { reportError, t } from '../../i18n';
import { keyStatusOf, presetInfo, refreshKeyStatus, removeKey, saveKey } from '../../providers';
import type { ProviderEntry } from '../../../shared/types';

export function ProviderKeyField(props: { entry: ProviderEntry }) {
  let input: HTMLInputElement | undefined;
  const [draft, setDraft] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const status = () => keyStatusOf(props.entry.id);

  onMount(() => {
    refreshKeyStatus(props.entry.id).catch((err: unknown) => reportError('error.providerKey', err));
  });

  const save = async () => {
    const key = draft().trim();
    if (!key || busy()) return;
    setBusy(true);
    try {
      await saveKey(props.entry.id, key);
      setDraft('');
      if (input) input.value = '';
      showToast({ kind: 'success', title: t('settings.providers.key.done') });
    } catch (err) {
      reportError('error.providerKey', err);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      await removeKey(props.entry.id);
    } catch (err) {
      reportError('error.providerKey', err);
    } finally {
      setBusy(false);
    }
  };

  const describe = () => {
    const current = status();
    if (current?.hasKey) {
      return current.last4
        ? t('settings.providers.key.saved', { last4: current.last4 })
        : t('settings.providers.key.savedShort');
    }
    return presetInfo(props.entry.preset).needsKey
      ? t('settings.providers.key.none')
      : t('settings.providers.key.optional');
  };

  return (
    <div class="af-key-field">
      <input
        ref={input}
        id={`af-key-${props.entry.id}`}
        class="af-input"
        type="password"
        autocomplete="off"
        spellcheck={false}
        placeholder={t('settings.providers.key.placeholder')}
        onInput={(event) => setDraft(event.currentTarget.value)}
        onKeyDown={(event) => event.key === 'Enter' && void save()}
      />
      <button
        type="button"
        class="af-button small"
        disabled={!draft().trim() || busy()}
        onClick={() => void save()}
      >
        {t('settings.providers.key.save')}
      </button>
      <span class="af-key-status" classList={{ ok: status()?.hasKey === true }}>
        {describe()}
      </span>
      <Show when={status()?.hasKey}>
        <button
          type="button"
          class="af-button secondary small"
          disabled={busy()}
          onClick={() => void remove()}
        >
          {t('settings.providers.key.remove')}
        </button>
      </Show>
    </div>
  );
}
