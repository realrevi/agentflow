// One provider entry: type/preset, name, base URL, key, default model, limits, and a
// "Test connection" button. Edits go to settings (debounced); the key to the keychain.
import { createSignal, For, Show } from 'solid-js';
import { toAppError } from '../../../shared/errors';
import { ipc } from '../../../shared/ipc';
import type { ProviderEntry, ProviderPreset } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import {
  baseUrlProblem,
  forgetModels,
  loadModels,
  MAX_OUTPUT_TOKENS,
  modelsOf,
  presetInfo,
  presetLabel,
  PRESETS,
  TEMPERATURE,
} from '../../providers';
import { flushSettings, updateSettings } from '../settingsStore';
import { ProviderKeyField } from './ProviderKeyField';
import { Row } from './Section';

const URL_PROBLEM: Record<'empty' | 'invalid' | 'insecure', I18nKey> = {
  empty: 'settings.providers.baseUrl.empty',
  invalid: 'settings.providers.baseUrl.invalid',
  insecure: 'settings.providers.baseUrl.insecure',
};

type Test = { state: 'idle' | 'testing' | 'ok' | 'error'; text: string };

export function ProviderEditor(props: { entry: ProviderEntry }) {
  const id = () => props.entry.id;
  const field = (name: string) => `af-provider-${id()}-${name}`;
  const [test, setTest] = createSignal<Test>({ state: 'idle', text: '' });
  const edit = (change: (entry: ProviderEntry) => void) =>
    updateSettings((s) => {
      const entry = s.providers.find((p) => p.id === id());
      if (entry) change(entry);
    });

  const changePreset = (preset: ProviderPreset) => {
    const before = presetInfo(props.entry.preset);
    const after = presetInfo(preset);
    edit((entry) => {
      // Follow the preset's URL unless the user typed their own.
      if (!entry.baseUrl.trim() || entry.baseUrl === before.baseUrl) entry.baseUrl = after.baseUrl;
      entry.preset = preset;
      entry.kind = after.kind;
    });
    forgetModels(id());
  };

  const runTest = async () => {
    setTest({ state: 'testing', text: t('settings.providers.testing') });
    try {
      await flushSettings();
      const result = await ipc.providerTest(id());
      setTest({
        state: 'ok',
        text:
          result.method === 'models'
            ? t('settings.providers.test.models', { count: result.models })
            : t('settings.providers.test.request'),
      });
    } catch (err) {
      setTest({ state: 'error', text: toAppError(err).message });
    }
  };

  const fetchModels = async () => {
    await flushSettings();
    await loadModels(id(), true);
  };

  const urlProblem = () => baseUrlProblem(props.entry.baseUrl);
  const models = () => modelsOf(id());

  return (
    <div class="af-provider-editor">
      <Row label={t('settings.providers.preset')} for={field('preset')}>
        <select
          id={field('preset')}
          class="af-select"
          value={props.entry.preset}
          onChange={(event) => changePreset(event.currentTarget.value as ProviderPreset)}
        >
          <optgroup label={t('settings.providers.group.anthropic')}>
            <option value="anthropic">{presetLabel('anthropic')}</option>
          </optgroup>
          <optgroup label={t('settings.providers.group.openai')}>
            <For each={PRESETS.filter((preset) => preset.kind === 'openai')}>
              {(preset) => <option value={preset.id}>{presetLabel(preset.id)}</option>}
            </For>
          </optgroup>
        </select>
      </Row>
      <Row label={t('settings.providers.name')} for={field('name')}>
        <input
          id={field('name')}
          class="af-input"
          value={props.entry.name}
          placeholder={presetLabel(props.entry.preset)}
          maxLength={64}
          onInput={(event) => {
            const name = event.currentTarget.value;
            edit((entry) => (entry.name = name));
          }}
        />
      </Row>
      <Row label={t('settings.providers.baseUrl')} for={field('url')}>
        <input
          id={field('url')}
          class="af-input af-input-wide"
          value={props.entry.baseUrl}
          spellcheck={false}
          aria-invalid={urlProblem() !== null}
          onChange={(event) => {
            const url = event.currentTarget.value.trim();
            edit((entry) => (entry.baseUrl = url));
            forgetModels(id());
          }}
        />
        <Show when={urlProblem()}>
          {(problem) => <span class="af-field-error">{t(URL_PROBLEM[problem()])}</span>}
        </Show>
      </Row>
      <Row label={t('settings.providers.key')} for={`af-key-${id()}`}>
        <ProviderKeyField entry={props.entry} />
      </Row>
      <Row label={t('settings.providers.defaultModel')} for={field('model')}>
        <input
          id={field('model')}
          class="af-input"
          value={props.entry.defaultModel}
          list={field('models')}
          spellcheck={false}
          placeholder={t('settings.providers.defaultModel.placeholder')}
          onChange={(event) => {
            const model = event.currentTarget.value.trim();
            edit((entry) => (entry.defaultModel = model));
          }}
        />
        <datalist id={field('models')}>
          <For each={models().list}>
            {(model) => <option value={model.id}>{model.name ?? ''}</option>}
          </For>
        </datalist>
        <button
          type="button"
          class="af-button secondary small"
          disabled={models().loading}
          onClick={() => void fetchModels()}
        >
          {t('settings.providers.loadModels')}
        </button>
        <Show when={models().error}>
          {(error) => <span class="af-field-error">{error()}</span>}
        </Show>
      </Row>
      <Row label={t('settings.providers.maxTokens')} for={field('max')}>
        <input
          id={field('max')}
          class="af-input af-input-number"
          type="number"
          min={1}
          max={MAX_OUTPUT_TOKENS}
          value={props.entry.maxTokens}
          onChange={(event) => {
            const value = event.currentTarget.valueAsNumber;
            if (Number.isInteger(value) && value >= 1 && value <= MAX_OUTPUT_TOKENS) {
              edit((entry) => (entry.maxTokens = value));
            }
          }}
        />
      </Row>
      <Row label={t('settings.providers.temperature')} for={field('temp')}>
        <input
          id={field('temp')}
          class="af-input af-input-number"
          type="number"
          min={TEMPERATURE.min}
          max={TEMPERATURE.max}
          step={TEMPERATURE.step}
          value={props.entry.temperature ?? ''}
          placeholder={t('settings.providers.temperature.placeholder')}
          onChange={(event) => {
            const raw = event.currentTarget.value.trim();
            const value = event.currentTarget.valueAsNumber;
            if (raw === '') edit((entry) => (entry.temperature = null));
            else if (
              Number.isFinite(value) &&
              value >= TEMPERATURE.min &&
              value <= TEMPERATURE.max
            ) {
              edit((entry) => (entry.temperature = value));
            }
          }}
        />
        <span class="af-settings-hint inline">{t('settings.providers.temperature.hint')}</span>
      </Row>
      <div class="af-provider-test">
        <button
          type="button"
          class="af-button secondary small"
          disabled={test().state === 'testing'}
          onClick={() => void runTest()}
        >
          {t('settings.providers.test')}
        </button>
        <Show when={test().state !== 'idle'}>
          <span class={`af-test-result ${test().state}`} role="status">
            {test().text}
          </span>
        </Show>
      </div>
    </div>
  );
}
