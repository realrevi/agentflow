// Step 1: pick a template: a built-in starter, a template file, or a draft made with AI.
import { createResource, createSignal, For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { AgentTemplate } from '../../../shared/types';
import { reportError, t } from '../../i18n';
import { CreateWithAi } from './CreateWithAi';
import type { Wizard } from './wizardStore';

export function StepTemplate(props: { wizard: Wizard; workspace: string | null }) {
  const [starters] = createResource(() => ipc.agentsStarters().catch(() => [] as AgentTemplate[]));
  const [creating, setCreating] = createSignal(false);

  const importFile = async () => {
    try {
      const path = await ipc.pickJsonFile(t('installer.import'));
      if (path) await props.wizard.start(await ipc.agentsTemplateRead(path));
    } catch (err) {
      reportError('error.installerImport', err);
    }
  };

  return (
    <section class="af-wizard-step">
      <div class="af-wizard-actions start">
        <button type="button" class="af-button secondary" onClick={() => void importFile()}>
          {t('installer.import')}
        </button>
        <button type="button" class="af-button secondary" onClick={() => setCreating(!creating())}>
          {t('installer.createWithAi')}
        </button>
      </div>
      <Show when={creating()}>
        <CreateWithAi
          workspace={props.workspace}
          onDraft={(template) => {
            setCreating(false);
            void props.wizard.start(template);
          }}
        />
      </Show>
      <h3 class="af-installer-subtitle">{t('installer.starters')}</h3>
      <div class="af-starter-grid">
        <For each={starters() ?? []}>
          {(template) => (
            <button
              type="button"
              class="af-starter"
              onClick={() => void props.wizard.start(template)}
            >
              <strong>{template.name}</strong>
              <span>{template.description}</span>
              <span class="af-starter-tags">{(template.tags ?? []).join(' · ')}</span>
            </button>
          )}
        </For>
      </div>
    </section>
  );
}
