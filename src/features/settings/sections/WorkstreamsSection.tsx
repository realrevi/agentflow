// Workstreams, the attention glow and the Agent Stage (phase 9).
import { For } from 'solid-js';
import type { HandoffMode } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import { MAX_ROUNDS, REPORT_CAP_KB, STAGE_SCALES } from '../defaults';
import { settings, updateSettings } from '../settingsStore';
import { ColorField } from './ColorField';
import { Row, Section } from './Section';

const MODES: readonly { mode: HandoffMode; label: I18nKey }[] = [
  { mode: 'manual', label: 'workstream.mode.manual' },
  { mode: 'semiAuto', label: 'workstream.mode.semiAuto' },
  { mode: 'auto', label: 'workstream.mode.auto' },
];

function clamp(value: number, range: { min: number; max: number }): number | null {
  return Number.isFinite(value)
    ? Math.min(range.max, Math.max(range.min, Math.round(value)))
    : null;
}

export function WorkstreamsSection() {
  const ws = () => settings.workstreams;
  return (
    <Section title={t('settings.workstreams')} hint={t('settings.workstreams.hint')}>
      <Row label={t('settings.workstreams.glow')} for="af-ws-glow">
        <input
          id="af-ws-glow"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={ws().glow}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((s) => (s.workstreams.glow = on));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.glowColor')} for="af-ws-glow-color">
        <ColorField
          id="af-ws-glow-color"
          value={ws().glowColor}
          fallback={settings.appearance.agentBold.color}
          resetLabel={t('settings.workstreams.glowColor.reset')}
          onChange={(color) => updateSettings((s) => (s.workstreams.glowColor = color))}
        />
      </Row>
      <Row label={t('settings.workstreams.notify')} for="af-ws-notify">
        <input
          id="af-ws-notify"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={ws().notify}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((s) => (s.workstreams.notify = on));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.defaultMode')} for="af-ws-mode">
        <select
          id="af-ws-mode"
          class="af-select"
          value={ws().defaultMode}
          onChange={(e) => {
            const mode = MODES.find((m) => m.mode === e.currentTarget.value)?.mode;
            if (mode) updateSettings((s) => (s.workstreams.defaultMode = mode));
          }}
        >
          <For each={MODES}>{(m) => <option value={m.mode}>{t(m.label)}</option>}</For>
        </select>
      </Row>
      <Row label={t('settings.workstreams.maxRounds')} for="af-ws-rounds">
        <input
          id="af-ws-rounds"
          class="af-input"
          type="number"
          min={MAX_ROUNDS.min}
          max={MAX_ROUNDS.max}
          value={ws().maxRounds}
          onChange={(e) => {
            const value = clamp(e.currentTarget.valueAsNumber, MAX_ROUNDS);
            if (value !== null) updateSettings((s) => (s.workstreams.maxRounds = value));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.reportCap')} for="af-ws-cap">
        <input
          id="af-ws-cap"
          class="af-input"
          type="number"
          min={REPORT_CAP_KB.min}
          max={REPORT_CAP_KB.max}
          value={ws().reportCapKb}
          onChange={(e) => {
            const value = clamp(e.currentTarget.valueAsNumber, REPORT_CAP_KB);
            if (value !== null) updateSettings((s) => (s.workstreams.reportCapKb = value));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.stageScale')} for="af-ws-scale">
        <select
          id="af-ws-scale"
          class="af-select"
          value={ws().stageScale}
          onChange={(e) => {
            const value = Number(e.currentTarget.value);
            const scale = STAGE_SCALES.find((option) => option === value);
            if (scale) updateSettings((s) => (s.workstreams.stageScale = scale));
          }}
        >
          <For each={STAGE_SCALES}>{(scale) => <option value={scale}>{`${scale}×`}</option>}</For>
        </select>
      </Row>
      <Row label={t('settings.workstreams.stageAnimations')} for="af-ws-anim">
        <input
          id="af-ws-anim"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={ws().stageAnimations}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((s) => (s.workstreams.stageAnimations = on));
          }}
        />
      </Row>
    </Section>
  );
}
