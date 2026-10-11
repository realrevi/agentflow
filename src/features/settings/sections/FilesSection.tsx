import { t } from '../../i18n';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';

export function FilesSection() {
  return (
    <Section title={t('settings.files')}>
      <Row label={t('settings.files.showHidden')} for="af-files-hidden">
        <input
          id="af-files-hidden"
          type="checkbox"
          class="af-switch"
          role="switch"
          checked={settings.files.showHidden}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.files.showHidden = on));
          }}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.files.showHiddenHint')}</p>
    </Section>
  );
}
