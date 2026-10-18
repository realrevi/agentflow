import { t } from '../../i18n';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';
import { Toggle } from '../../../shared/ui/Field';

export function FilesSection() {
  return (
    <Section title={t('settings.files')}>
      <Row label={t('settings.files.showHidden')} for="af-files-hidden">
        <Toggle
          id="af-files-hidden"
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
