// Edit a template's name, description and instructions before installing (an AI draft or
// a starter). The backend validates on preview, so a broken placeholder shows there.
import { Show, createSignal } from 'solid-js';
import { t } from '../../i18n';
import type { Wizard } from './wizardStore';
import { Button } from '../../../shared/ui/Button';
import { Input, Textarea } from '../../../shared/ui/Field';

export function TemplateEditor(props: { wizard: Wizard }) {
  const [open, setOpen] = createSignal(false);
  const edit = (field: 'name' | 'description' | 'instructions', value: string) => {
    const current = props.wizard.template();
    if (current) props.wizard.setTemplate({ ...current, [field]: value });
  };
  return (
    <div class="af-template-editor">
      <Button variant="ghost" size="sm" aria-expanded={open()} onClick={() => setOpen(!open())}>
        {t('installer.edit')}
      </Button>
      <Show when={open() && props.wizard.template()}>
        {(template) => (
          <div class="af-template-fields">
            <label class="af-question-label" for="af-tpl-name">
              {t('installer.field.name')}
            </label>
            <Input
              id="af-tpl-name"
              value={template().name}
              onChange={(e) => edit('name', e.currentTarget.value)}
            />
            <label class="af-question-label" for="af-tpl-description">
              {t('installer.field.description')}
            </label>
            <Input
              id="af-tpl-description"
              value={template().description}
              onChange={(e) => edit('description', e.currentTarget.value)}
            />
            <label class="af-question-label" for="af-tpl-instructions">
              {t('installer.field.instructions')}
            </label>
            <Textarea
              class="af-template-body"
              id="af-tpl-instructions"
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
