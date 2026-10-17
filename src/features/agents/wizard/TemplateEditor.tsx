// Edit a template's name, description and instructions before installing (an AI draft or
// a starter). The backend validates on preview, so a broken placeholder shows there.
import { Show, createSignal } from 'solid-js';
import { t } from '../../i18n';
import type { Wizard } from './wizardStore';

export function TemplateEditor(props: { wizard: Wizard }) {
  const [open, setOpen] = createSignal(false);
  const edit = (field: 'name' | 'description' | 'instructions', value: string) => {
    const current = props.wizard.template();
    if (current) props.wizard.setTemplate({ ...current, [field]: value });
  };
  return (
    <div class="af-template-editor">
      <button
        type="button"
        class="af-link-btn"
        aria-expanded={open()}
        onClick={() => setOpen(!open())}
      >
        {t('installer.edit')}
      </button>
      <Show when={open() && props.wizard.template()}>
        {(template) => (
          <div class="af-template-fields">
            <label class="af-question-label" for="af-tpl-name">
              {t('installer.field.name')}
            </label>
            <input
              id="af-tpl-name"
              class="af-input"
              value={template().name}
              onChange={(e) => edit('name', e.currentTarget.value)}
            />
            <label class="af-question-label" for="af-tpl-description">
              {t('installer.field.description')}
            </label>
            <input
              id="af-tpl-description"
              class="af-input"
              value={template().description}
              onChange={(e) => edit('description', e.currentTarget.value)}
            />
            <label class="af-question-label" for="af-tpl-instructions">
              {t('installer.field.instructions')}
            </label>
            <textarea
              id="af-tpl-instructions"
              class="af-input af-template-body"
              rows={12}
              value={template().instructions}
              onChange={(e) => edit('instructions', e.currentTarget.value)}
            />
            <p class="af-installer-hint">{t('installer.field.instructionsHint')}</p>
          </div>
        )}
      </Show>
    </div>
  );
}
