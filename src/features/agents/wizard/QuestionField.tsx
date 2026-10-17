// One template question as a form field, by type. Detected answers carry a "detected" mark
// until the user changes them.
import { For, Match, Show, Switch } from 'solid-js';
import type { AnswerValue, Question } from '../../../shared/types';
import { t } from '../../i18n';

const HEX = /#[0-9a-fA-F]{3,8}\b/g;

function asText(value: AnswerValue | undefined): string {
  if (Array.isArray(value)) return value.join(', ');
  return typeof value === 'string' ? value : '';
}

export function QuestionField(props: {
  question: Question;
  value: AnswerValue | undefined;
  detected: boolean;
  onChange: (value: AnswerValue) => void;
}) {
  const id = () => `af-q-${props.question.id}`;
  const text = () => asText(props.value);
  const list = () => (Array.isArray(props.value) ? props.value : []);
  return (
    <div class="af-question">
      <label class="af-question-label" for={id()}>
        {props.question.label}
        <Show when={props.question.required}>
          <span class="af-question-required" aria-hidden="true">
            {' *'}
          </span>
        </Show>
        <Show when={props.detected}>
          <span class="af-badge added" title={t('installer.detectedHint')}>
            {t('installer.detected')}
          </span>
        </Show>
      </label>
      <Switch
        fallback={
          <input
            id={id()}
            class="af-input"
            value={text()}
            onInput={(e) => props.onChange(e.currentTarget.value)}
          />
        }
      >
        <Match when={props.question.type === 'multiline'}>
          <textarea
            id={id()}
            class="af-input"
            rows={3}
            value={text()}
            onInput={(e) => props.onChange(e.currentTarget.value)}
          />
        </Match>
        <Match when={props.question.type === 'boolean'}>
          <input
            id={id()}
            type="checkbox"
            class="af-switch"
            role="switch"
            checked={props.value === true}
            onChange={(e) => props.onChange(e.currentTarget.checked)}
          />
        </Match>
        <Match when={props.question.type === 'choice'}>
          <select
            id={id()}
            class="af-select"
            value={text()}
            onChange={(e) => props.onChange(e.currentTarget.value)}
          >
            <For each={props.question.options ?? []}>
              {(option) => <option value={option}>{option}</option>}
            </For>
          </select>
        </Match>
        <Match when={props.question.type === 'multiChoice'}>
          <div id={id()} class="af-question-options" role="group">
            <For each={props.question.options ?? []}>
              {(option) => (
                <label class="af-check">
                  <input
                    type="checkbox"
                    checked={list().includes(option)}
                    onChange={(e) => {
                      const on = e.currentTarget.checked;
                      const next = (props.question.options ?? []).filter((o) =>
                        o === option ? on : list().includes(o),
                      );
                      props.onChange(next);
                    }}
                  />
                  {option}
                </label>
              )}
            </For>
          </div>
        </Match>
        <Match when={props.question.type === 'color'}>
          <div class="af-question-color">
            <input
              id={id()}
              class="af-input"
              value={text()}
              placeholder={t('installer.colorPlaceholder')}
              onInput={(e) => props.onChange(e.currentTarget.value)}
            />
            <input
              type="color"
              class="af-question-picker"
              aria-label={t('installer.addColor')}
              title={t('installer.addColor')}
              onChange={(e) => {
                const added = e.currentTarget.value;
                props.onChange(text().trim() ? `${text().trim()}, ${added}` : added);
              }}
            />
          </div>
          <div class="af-question-swatches" aria-hidden="true">
            <For each={text().match(HEX) ?? []}>
              {(hex) => <span class="af-swatch" style={{ background: hex }} title={hex} />}
            </For>
          </div>
        </Match>
        <Match when={props.question.type === 'font'}>
          <input
            id={id()}
            class="af-input"
            value={text()}
            onInput={(e) => props.onChange(e.currentTarget.value)}
          />
          <Show when={text().trim()}>
            <p class="af-question-font" style={{ 'font-family': text() }}>
              {t('installer.fontSample')}
            </p>
          </Show>
        </Match>
      </Switch>
      <Show when={props.question.help}>{(help) => <p class="af-installer-hint">{help()}</p>}</Show>
    </div>
  );
}
