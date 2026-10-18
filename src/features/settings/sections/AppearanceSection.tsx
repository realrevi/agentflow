import { createMemo, For, Show } from 'solid-js';
import { cssVar, mapThemeColors } from '../../theme/mapping';
import { DEFAULT_BOLD_COLOR } from '../../theme/overrides';
import { activeTheme, importThemeFile, selectTheme, themes } from '../../theme/themeStore';
import { t } from '../../i18n';
import { settings, updateSettings } from '../settingsStore';
import { ColorField } from './ColorField';
import { Row, Section } from './Section';
import { Button } from '../../../shared/ui/Button';
import { Select, Toggle } from '../../../shared/ui/Field';

export function AppearanceSection() {
  let fileInput: HTMLInputElement | undefined;

  const builtin = () => themes().filter((theme) => theme.builtin);
  const imported = () => themes().filter((theme) => !theme.builtin);
  // Theme colors without user overrides, shown while an override is not set.
  const themeVars = createMemo(() => {
    const theme = activeTheme();
    return theme ? mapThemeColors(theme.colors, theme.kind) : {};
  });
  const themeColor = (name: string) => themeVars()[cssVar(name)] ?? '#888888';
  const boldOverride = () => {
    const color = settings.appearance.agentBold.color;
    return color.toLowerCase() === DEFAULT_BOLD_COLOR.toLowerCase() ? null : color;
  };

  const onFile = (event: Event & { currentTarget: HTMLInputElement }) => {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = '';
    if (file) void importThemeFile(file);
  };

  return (
    <Section title={t('settings.appearance')}>
      <Row label={t('settings.theme')} for="af-theme">
        <Select
          id="af-theme"
          value={settings.appearance.themeId}
          onChange={(event) => selectTheme(event.currentTarget.value)}
        >
          <optgroup label={t('settings.theme.builtin')}>
            <For each={builtin()}>{(theme) => <option value={theme.id}>{theme.name}</option>}</For>
          </optgroup>
          <Show when={imported().length > 0}>
            <optgroup label={t('settings.theme.custom')}>
              <For each={imported()}>
                {(theme) => <option value={theme.id}>{theme.name}</option>}
              </For>
            </optgroup>
          </Show>
        </Select>
        <Button variant="secondary" onClick={() => fileInput?.click()}>
          {t('settings.theme.import')}
        </Button>
        <input
          ref={fileInput}
          type="file"
          accept=".json,application/json"
          hidden
          onChange={onFile}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.theme.importHint')}</p>

      <Row label={t('settings.textColor')} for="af-text-color">
        <ColorField
          id="af-text-color"
          value={settings.appearance.textColor}
          fallback={themeColor('fg')}
          onChange={(color) => updateSettings((s) => (s.appearance.textColor = color))}
        />
      </Row>
      <Row label={t('settings.accentColor')} for="af-accent-color">
        <ColorField
          id="af-accent-color"
          value={settings.appearance.accentColor}
          fallback={themeColor('accent')}
          onChange={(color) => updateSettings((s) => (s.appearance.accentColor = color))}
        />
      </Row>

      <h3 class="af-settings-subtitle">{t('settings.agentBold')}</h3>
      <p class="af-settings-hint">{t('settings.agentBold.hint')}</p>
      <Row label={t('settings.agentBold.color')} for="af-bold-color">
        <ColorField
          id="af-bold-color"
          value={boldOverride()}
          fallback={DEFAULT_BOLD_COLOR}
          resetLabel={t('settings.reset')}
          onChange={(color) =>
            updateSettings((s) => (s.appearance.agentBold.color = color ?? DEFAULT_BOLD_COLOR))
          }
        />
      </Row>
      <Row label={t('settings.agentBold.glow')} for="af-bold-glow">
        <Toggle
          id="af-bold-glow"
          checked={settings.appearance.agentBold.glow}
          onChange={(event) => {
            const glow = event.currentTarget.checked;
            updateSettings((s) => (s.appearance.agentBold.glow = glow));
          }}
        />
      </Row>
      <Row label={t('settings.agentBold.preview')}>
        <p class="af-bold-preview">
          <span class="af-agent-bold">{t('settings.agentBold.previewName')}</span>{' '}
          {t('settings.agentBold.previewText')}
        </p>
      </Row>
    </Section>
  );
}
