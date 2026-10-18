// Step 5: what was written (project files open in the editor) and how to use the agent in
// each tool.
import { For, Show } from 'solid-js';
import { t } from '../../i18n';
import { hintFor, TARGETS } from './answers';
import type { Wizard } from './wizardStore';
import { Button } from '../../../shared/ui/Button';

export function StepDone(props: { wizard: Wizard; onOpenFile: (path: string) => void }) {
  const w = props.wizard;
  const written = () => (w.result()?.plan.files ?? []).filter((f) => !f.remove);
  return (
    <section class="af-wizard-step">
      <Show when={w.result()}>
        {(result) => (
          <>
            <h3 class="af-installer-subtitle">
              {t('installer.done', { name: result().installed.template.name })}
            </h3>
            <ul class="af-plan-written">
              <For each={written()}>
                {(file) => (
                  <li>
                    <Show when={file.scope === 'project'} fallback={<code>{file.display}</code>}>
                      <Button variant="ghost" size="sm" onClick={() => props.onOpenFile(file.path)}>
                        {file.display}
                      </Button>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
            <h3 class="af-installer-subtitle">{t('installer.howToUse')}</h3>
            <ul class="af-plan-hints">
              <For
                each={TARGETS.filter((target) => result().installed.targets.includes(target.id))}
              >
                {(target) => (
                  <li>
                    <strong>{t(target.label)}:</strong>{' '}
                    {t(hintFor(result().installed.template, target), {
                      id: result().installed.id,
                      name: result().installed.template.name,
                    })}
                  </li>
                )}
              </For>
            </ul>
          </>
        )}
      </Show>
      <div class="af-wizard-actions">
        <Button onClick={() => w.reset()}>{t('installer.another')}</Button>
      </div>
    </section>
  );
}
