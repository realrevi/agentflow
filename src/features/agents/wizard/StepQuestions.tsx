// Step 3: the template's questions, grouped, prefilled (detected ones marked).
import { For, Show } from 'solid-js';
import { t } from '../../i18n';
import { groups, missing } from './answers';
import { QuestionField } from './QuestionField';
import type { Wizard } from './wizardStore';

export function StepQuestions(props: { wizard: Wizard }) {
  const w = props.wizard;
  const questions = () => w.template()?.questions ?? [];
  const lacking = () => {
    const template = w.template();
    return template ? missing(template, w.answers) : [];
  };
  return (
    <section class="af-wizard-step">
      <Show
        when={questions().length > 0}
        fallback={<p class="af-installer-hint">{t('installer.noQuestions')}</p>}
      >
        <For each={groups(questions())}>
          {(group) => (
            <fieldset class="af-question-group">
              <Show when={group.name}>
                <legend>{group.name}</legend>
              </Show>
              <For each={group.questions}>
                {(question) => (
                  <QuestionField
                    question={question}
                    value={w.answers[question.id]}
                    detected={w.detected().has(question.id)}
                    onChange={(value) => w.setAnswer(question.id, value)}
                  />
                )}
              </For>
            </fieldset>
          )}
        </For>
      </Show>
      <Show when={lacking().length > 0}>
        <p class="af-installer-warning">
          {t('installer.missing', { fields: lacking().join(', ') })}
        </p>
      </Show>
      <div class="af-wizard-actions">
        <button type="button" class="af-button secondary" onClick={() => w.setStep('targets')}>
          {t('installer.back')}
        </button>
        <button
          type="button"
          class="af-button"
          disabled={lacking().length > 0 || w.busy()}
          onClick={() => void w.preview()}
        >
          {t('installer.toPreview')}
        </button>
      </div>
    </section>
  );
}
