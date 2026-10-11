import { For } from 'solid-js';
import { LOCALES, t } from '../../i18n';
import { settings, updateSettings } from '../settingsStore';
import { Section } from './Section';

export function LanguageSection() {
  return (
    <Section title={t('settings.language')} hint={t('settings.language.hint')}>
      <div class="af-choice-list" role="radiogroup" aria-label={t('settings.language')}>
        <For each={LOCALES}>
          {(entry) => (
            <label class="af-choice" classList={{ active: settings.language === entry.code }}>
              <input
                type="radio"
                name="af-language"
                value={entry.code}
                checked={settings.language === entry.code}
                onChange={() => updateSettings((s) => (s.language = entry.code))}
              />
              <span lang={entry.code}>{entry.name}</span>
            </label>
          )}
        </For>
      </div>
    </Section>
  );
}
