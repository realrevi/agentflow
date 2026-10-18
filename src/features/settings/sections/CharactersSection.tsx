import { For } from 'solid-js';
import type { AvatarScale, CharacterSettings } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import { ANIMATION_SPEED, AVATAR_SCALES } from '../defaults';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';
import { Select, Toggle } from '../../../shared/ui/Field';

type Toggle = { [K in keyof CharacterSettings]: CharacterSettings[K] extends boolean ? K : never };
type ToggleKey = Toggle[keyof CharacterSettings];

const TOGGLES: readonly { key: ToggleKey; label: I18nKey }[] = [
  { key: 'enabled', label: 'settings.characters.enabled' },
  { key: 'cursorFollow', label: 'settings.characters.cursorFollow' },
  { key: 'packet', label: 'settings.characters.packet' },
  { key: 'bubble', label: 'settings.characters.bubble' },
  { key: 'typewriter', label: 'settings.characters.typewriter' },
  { key: 'onShells', label: 'settings.characters.onShells' },
];

export function CharactersSection() {
  return (
    <Section title={t('settings.characters')} hint={t('settings.characters.hint')}>
      <For each={TOGGLES}>
        {(toggle) => (
          <Row label={t(toggle.label)} for={`af-chars-${toggle.key}`}>
            <Toggle
              id={`af-chars-${toggle.key}`}
              checked={settings.characters[toggle.key]}
              onChange={(event) => {
                const on = event.currentTarget.checked;
                updateSettings((s) => (s.characters[toggle.key] = on));
              }}
            />
          </Row>
        )}
      </For>
      <Row label={t('settings.characters.speed')} for="af-chars-speed">
        <input
          id="af-chars-speed"
          class="af-range"
          type="range"
          min={ANIMATION_SPEED.min}
          max={ANIMATION_SPEED.max}
          step={ANIMATION_SPEED.step}
          value={settings.characters.speed}
          onInput={(event) => {
            const speed = event.currentTarget.valueAsNumber;
            if (Number.isFinite(speed)) updateSettings((s) => (s.characters.speed = speed));
          }}
        />
        <span class="af-range-value">{`${settings.characters.speed}×`}</span>
      </Row>
      <Row label={t('settings.characters.scale')} for="af-chars-scale">
        <Select
          id="af-chars-scale"
          value={settings.characters.scale}
          onChange={(event) => {
            const value = Number(event.currentTarget.value);
            const scale = AVATAR_SCALES.find((option): option is AvatarScale => option === value);
            if (scale) updateSettings((s) => (s.characters.scale = scale));
          }}
        >
          <For each={AVATAR_SCALES}>{(scale) => <option value={scale}>{`${scale}×`}</option>}</For>
        </Select>
      </Row>
    </Section>
  );
}
