// Step 2: which tools to install for, and where (the project, or the documented home
// folders after a separate confirmation in the preview). Also lets the template be edited
// (e.g. an AI draft) and exported.
import { For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import { showToast } from '../../../shared/toast';
import { reportError, t } from '../../i18n';
import { supportsTarget, TARGETS } from './answers';
import { TemplateEditor } from './TemplateEditor';
import type { Wizard } from './wizardStore';
import { Button } from '../../../shared/ui/Button';
import { Checkbox } from '../../../shared/ui/Field';

export function StepTargets(props: { wizard: Wizard }) {
  const w = props.wizard;
  const existing = (target: string) =>
    (w.detection()?.existing ?? []).filter((file) => file.target === target).map((f) => f.path);
  const supported = (id: (typeof TARGETS)[number]['id']) => {
    const template = w.template();
    return template !== null && supportsTarget(template, id);
  };
  const usable = (target: (typeof TARGETS)[number]) =>
    supported(target.id) && (w.scope() === 'project' || target.userScope);
  const chosen = () => TARGETS.filter((target) => w.targets().has(target.id) && usable(target));

  const exportTemplate = async () => {
    const template = w.template();
    if (!template) return;
    try {
      const path = await ipc.pickJsonSavePath(t('installer.export'), `${template.id}.json`);
      if (!path) return;
      await ipc.agentsTemplateWrite(path, template);
      showToast({ kind: 'success', title: t('installer.exported') });
    } catch (err) {
      reportError('error.installer', err);
    }
  };

  return (
    <section class="af-wizard-step">
      <Show when={w.template()}>
        {(template) => (
          <div class="af-wizard-template">
            <strong>{template().name}</strong>
            <span class="af-badge">v{template().version}</span>
            <Button variant="ghost" size="sm" onClick={() => void exportTemplate()}>
              {t('installer.export')}
            </Button>
            <TemplateEditor wizard={w} />
          </div>
        )}
      </Show>
      <fieldset class="af-question-group">
        <legend>{t('installer.targets')}</legend>
        <For each={TARGETS}>
          {(target) => (
            <label class="af-check af-target" classList={{ disabled: !usable(target) }}>
              <Checkbox
                disabled={!usable(target)}
                checked={w.targets().has(target.id) && usable(target)}
                onChange={(e) => w.toggleTarget(target.id, e.currentTarget.checked)}
              />
              <span>{t(target.label)}</span>
              <Show when={existing(target.id).length > 0}>
                <span class="af-target-existing">
                  {t('installer.existing', { files: existing(target.id).join(', ') })}
                </span>
              </Show>
              <Show when={!supported(target.id)}>
                <span class="af-target-existing">{t('installer.notForKind')}</span>
              </Show>
              <Show when={supported(target.id) && !usable(target)}>
                <span class="af-target-existing">{t('installer.projectOnly')}</span>
              </Show>
            </label>
          )}
        </For>
      </fieldset>
      <fieldset class="af-question-group">
        <legend>{t('installer.scope')}</legend>
        <label class="af-check">
          <input
            type="radio"
            name="af-scope"
            checked={w.scope() === 'project'}
            onChange={() => w.setScope('project')}
          />
          {t('installer.scope.project')}
        </label>
        <label class="af-check">
          <input
            type="radio"
            name="af-scope"
            checked={w.scope() === 'user'}
            onChange={() => w.setScope('user')}
          />
          {t('installer.scope.user')}
        </label>
        <Show when={w.scope() === 'user'}>
          <p class="af-installer-hint">{t('installer.scope.userHint')}</p>
        </Show>
      </fieldset>
      <div class="af-wizard-actions">
        <Button variant="secondary" onClick={() => w.reset()}>
          {t('installer.back')}
        </Button>
        <Button
          disabled={chosen().length === 0}
          onClick={() => {
            for (const target of TARGETS) {
              if (!usable(target)) w.toggleTarget(target.id, false);
            }
            w.setStep('questions');
          }}
        >
          {t('installer.next')}
        </Button>
      </div>
    </section>
  );
}
