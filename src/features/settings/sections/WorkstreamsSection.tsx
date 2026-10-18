// Workstreams, the attention glow and the Agent Stage (phase 9).
import { For } from 'solid-js';
import type { HandoffMode } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import { MAX_ROUNDS, REPORT_CAP_KB, STAGE_SCALES } from '../defaults';
import { settings, updateSettings } from '../settingsStore';
import { ColorField } from './ColorField';
import { Row, Section } from './Section';
import { Input, Select, Toggle } from '../../../shared/ui/Field';

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
        <Toggle
          id="af-ws-glow"
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
        <Toggle
          id="af-ws-notify"
          checked={ws().notify}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((s) => (s.workstreams.notify = on));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.defaultMode')} for="af-ws-mode">
        <Select
          id="af-ws-mode"
          value={ws().defaultMode}
          onChange={(e) => {
            const mode = MODES.find((m) => m.mode === e.currentTarget.value)?.mode;
            if (mode) updateSettings((s) => (s.workstreams.defaultMode = mode));
          }}
        >
          <For each={MODES}>{(m) => <option value={m.mode}>{t(m.label)}</option>}</For>
        </Select>
      </Row>
      <Row label={t('settings.workstreams.maxRounds')} for="af-ws-rounds">
        <Input
          id="af-ws-rounds"
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
        <Input
          id="af-ws-cap"
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
        <Select
          id="af-ws-scale"
          value={ws().stageScale}
          onChange={(e) => {
            const value = Number(e.currentTarget.value);
            const scale = STAGE_SCALES.find((option) => option === value);
            if (scale) updateSettings((s) => (s.workstreams.stageScale = scale));
          }}
        >
          <For each={STAGE_SCALES}>{(scale) => <option value={scale}>{`${scale}×`}</option>}</For>
        </Select>
      </Row>
      <Row
        label={t('settings.workstreams.autoRemind')}
        hint={t('settings.workstreams.autoRemindHint')}
        for="af-ws-remind"
      >
        <Toggle
          id="af-ws-remind"
          checked={ws().autoRemind}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((s) => (s.workstreams.autoRemind = on));
          }}
        />
      </Row>
      <Row label={t('settings.workstreams.stageAnimations')} for="af-ws-anim">
        <Toggle
          id="af-ws-anim"
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
