import { resetLayout } from '../../layout/layoutStore';
import { t } from '../../i18n';
import { Section } from './Section';

export function LayoutSection() {
  return (
    <Section title={t('settings.layout')} hint={t('settings.layout.resetHint')}>
      <button type="button" class="af-button secondary" onClick={resetLayout}>
        {t('settings.layout.reset')}
      </button>
    </Section>
  );
}
