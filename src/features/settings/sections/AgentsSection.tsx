// API agents: loop and tool limits, the default permission mode, the default system prompt,
// whether chats show reasoning and per-model "supports tools" overrides (models the app
// learned to run without tools are remembered by the backend, an override here wins).
import { For } from 'solid-js';
import type { PermissionMode } from '../../../shared/types';
import { t } from '../../i18n';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';

const LIMITS = {
  iterations: { min: 1, max: 200 },
  capKb: { min: 1, max: 1000 },
  timeout: { min: 5, max: 3600 },
} as const;

function numberIn(input: HTMLInputElement, limit: { min: number; max: number }): number | null {
  const value = input.valueAsNumber;
  return Number.isInteger(value) && value >= limit.min && value <= limit.max ? value : null;
}

export function AgentsSection() {
  const number = (
    id: string,
    label: string,
    limit: { min: number; max: number },
    value: () => number,
    apply: (value: number) => void,
  ) => (
    <Row label={label} for={id}>
      <input
        id={id}
        class="af-input"
        type="number"
        min={limit.min}
        max={limit.max}
        value={value()}
        onChange={(event) => {
          const next = numberIn(event.currentTarget, limit);
          if (next !== null) apply(next);
        }}
      />
    </Row>
  );

  return (
    <Section title={t('settings.agents')} hint={t('settings.agents.hint')}>
      <Row label={t('settings.agents.mode')} for="af-agents-mode">
        <select
          id="af-agents-mode"
          class="af-select"
          value={settings.agent.defaultMode}
          onChange={(event) => {
            const mode = event.currentTarget.value as PermissionMode;
            updateSettings((s) => (s.agent.defaultMode = mode));
          }}
        >
          <option value="ask">{t('agent.mode.ask')}</option>
          <option value="autoEdit">{t('agent.mode.autoEdit')}</option>
          <option value="readOnly">{t('agent.mode.readOnly')}</option>
        </select>
      </Row>
      <Row label={t('settings.agents.prompt')} for="af-agents-prompt">
        <input
          id="af-agents-prompt"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={settings.agent.defaultPrompt}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.agent.defaultPrompt = on));
          }}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.agents.promptHint')}</p>
      <Row label={t('settings.agents.reasoning')} for="af-agents-reasoning">
        <input
          id="af-agents-reasoning"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={settings.agent.showReasoning}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.agent.showReasoning = on));
          }}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.agents.reasoningHint')}</p>
      {number(
        'af-agents-iterations',
        t('settings.agents.iterations'),
        LIMITS.iterations,
        () => settings.agent.maxIterations,
        (value) => updateSettings((s) => (s.agent.maxIterations = value)),
      )}
      {number(
        'af-agents-cap',
        t('settings.agents.cap'),
        LIMITS.capKb,
        () => Math.round(settings.agent.toolResultCap / 1000),
        (value) => updateSettings((s) => (s.agent.toolResultCap = value * 1000)),
      )}
      {number(
        'af-agents-timeout',
        t('settings.agents.timeout'),
        LIMITS.timeout,
        () => settings.agent.commandTimeoutSecs,
        (value) => updateSettings((s) => (s.agent.commandTimeoutSecs = value)),
      )}
      <h3 class="af-settings-subtitle">{t('settings.agents.models')}</h3>
      <p class="af-settings-hint">{t('settings.agents.modelsHint')}</p>
      <For each={settings.agent.toolModels}>
        {(entry, index) => (
          <div class="af-settings-row">
            <input
              class="af-input"
              spellcheck={false}
              aria-label={t('settings.agents.model')}
              placeholder={t('settings.agents.model')}
              value={entry.model}
              onChange={(event) => {
                const model = event.currentTarget.value.trim();
                updateSettings((s) => {
                  const row = s.agent.toolModels[index()];
                  if (row) row.model = model;
                });
              }}
            />
            <select
              class="af-select"
              aria-label={t('settings.agents.tools')}
              value={entry.tools ? 'on' : 'off'}
              onChange={(event) => {
                const tools = event.currentTarget.value === 'on';
                updateSettings((s) => {
                  const row = s.agent.toolModels[index()];
                  if (row) row.tools = tools;
                });
              }}
            >
              <option value="on">{t('settings.agents.toolsOn')}</option>
              <option value="off">{t('settings.agents.toolsOff')}</option>
            </select>
            <button
              type="button"
              class="af-button secondary small"
              onClick={() => updateSettings((s) => s.agent.toolModels.splice(index(), 1))}
            >
              {t('settings.agents.remove')}
            </button>
          </div>
        )}
      </For>
      <button
        type="button"
        class="af-button secondary small"
        onClick={() => updateSettings((s) => s.agent.toolModels.push({ model: '', tools: false }))}
      >
        {t('settings.agents.add')}
      </button>
    </Section>
  );
}
