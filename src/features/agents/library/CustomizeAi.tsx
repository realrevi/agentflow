// "Customize with AI" for a library item: a configured provider adds project questions and
// a "Project specifics" section (the original text stays as it is); the result opens in
// the wizard for review. Usage goes to the ledger like "Create with AI".
import { createSignal, For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { AgentTemplate } from '../../../shared/types';
import { locale, reportError, t } from '../../i18n';
import { providerName } from '../../providers';
import { settings } from '../../settings/settingsStore';
import { Button } from '../../../shared/ui/Button';
import { Checkbox, Input, Select, Textarea } from '../../../shared/ui/Field';

export function CustomizeAi(props: {
  workspace: string | null;
  template: AgentTemplate;
  onDraft: (template: AgentTemplate) => void;
}) {
  const first = settings.providers[0];
  const [providerId, setProviderId] = createSignal(first?.id ?? '');
  const [model, setModel] = createSignal(first?.defaultModel ?? '');
  const [notes, setNotes] = createSignal('');
  const [ownLanguage, setOwnLanguage] = createSignal(false);
  const [busy, setBusy] = createSignal(false);

  const customize = async () => {
    setBusy(true);
    try {
      const result = await ipc.agentsGenerate({
        workspace: props.workspace,
        providerId: providerId(),
        model: model().trim(),
        description: notes().trim(),
        language: ownLanguage() ? locale() : 'en',
        base: props.template,
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
        <p class="af-installer-hint">{t('library.customize.hint')}</p>
        <label class="af-question-label" for="af-lib-notes">
          {t('library.customize.notes')}
        </label>
        <Textarea
          id="af-lib-notes"
          rows={3}
          placeholder={t('library.customize.placeholder')}
          value={notes()}
          onInput={(e) => setNotes(e.currentTarget.value)}
        />
        <div class="af-create-ai-row">
          <Select
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
          </Select>
          <Input
            aria-label={t('chat.model')}
            placeholder={t('chat.model.placeholder')}
            value={model()}
            onInput={(e) => setModel(e.currentTarget.value)}
          />
        </div>
        <label class="af-check">
          <Checkbox
            checked={ownLanguage()}
            onChange={(e) => setOwnLanguage(e.currentTarget.checked)}
          />
          {t('library.customize.ownLanguage')}
        </label>
        <Button disabled={busy() || !model().trim()} onClick={() => void customize()}>
          {busy() ? t('installer.ai.working') : t('library.customize.run')}
        </Button>
      </Show>
    </div>
  );
}
