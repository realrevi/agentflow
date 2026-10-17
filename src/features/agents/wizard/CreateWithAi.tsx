// "Create with AI": describe the agent, pick a configured provider and model; one request
// returns a template draft (validated, one automatic retry) that opens in the wizard for
// review. Nothing is installed without the normal preview.
import { createSignal, For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { AgentTemplate } from '../../../shared/types';
import { locale, reportError, t } from '../../i18n';
import { providerName } from '../../providers';
import { settings } from '../../settings/settingsStore';

export function CreateWithAi(props: {
  workspace: string | null;
  onDraft: (template: AgentTemplate) => void;
}) {
  const first = settings.providers[0];
  const [providerId, setProviderId] = createSignal(first?.id ?? '');
  const [model, setModel] = createSignal(first?.defaultModel ?? '');
  const [description, setDescription] = createSignal('');
  const [ownLanguage, setOwnLanguage] = createSignal(false);
  const [busy, setBusy] = createSignal(false);

  const generate = async () => {
    setBusy(true);
    try {
      const result = await ipc.agentsGenerate({
        workspace: props.workspace,
        providerId: providerId(),
        model: model().trim(),
        description: description().trim(),
        language: ownLanguage() ? locale() : 'en',
      });
      props.onDraft(result.template);
    } catch (err) {
      reportError('error.installerGenerate', err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div class="af-create-ai">
      <Show
        when={settings.providers.length > 0}
        fallback={<p class="af-installer-hint">{t('installer.ai.noProvider')}</p>}
      >
        <label class="af-question-label" for="af-ai-description">
          {t('installer.ai.describe')}
        </label>
        <textarea
          id="af-ai-description"
          class="af-input"
          rows={4}
          placeholder={t('installer.ai.placeholder')}
          value={description()}
          onInput={(e) => setDescription(e.currentTarget.value)}
        />
        <div class="af-create-ai-row">
          <select
            class="af-select"
            aria-label={t('chat.provider')}
            value={providerId()}
            onChange={(e) => {
              const id = e.currentTarget.value;
              setProviderId(id);
              setModel(settings.providers.find((p) => p.id === id)?.defaultModel ?? '');
            }}
          >
            <For each={settings.providers}>
              {(entry) => <option value={entry.id}>{providerName(entry)}</option>}
            </For>
          </select>
          <input
            class="af-input"
            aria-label={t('chat.model')}
            placeholder={t('chat.model.placeholder')}
            value={model()}
            onInput={(e) => setModel(e.currentTarget.value)}
          />
        </div>
        <fieldset class="af-question-group">
          <legend>{t('installer.ai.language')}</legend>
          <label class="af-check">
            <input
              type="radio"
              name="af-ai-language"
              checked={!ownLanguage()}
              onChange={() => setOwnLanguage(false)}
            />
            {t('installer.ai.english')}
          </label>
          <label class="af-check">
            <input
              type="radio"
              name="af-ai-language"
              checked={ownLanguage()}
              onChange={() => setOwnLanguage(true)}
            />
            {t('installer.ai.own')}
          </label>
        </fieldset>
        <button
          type="button"
          class="af-button"
          disabled={busy() || !description().trim() || !model().trim()}
          onClick={() => void generate()}
        >
          {busy() ? t('installer.ai.working') : t('installer.ai.generate')}
        </button>
      </Show>
    </div>
  );
}
