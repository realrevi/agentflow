import { resetLayout } from '../../layout/layoutStore';
import { t } from '../../i18n';
import { Section } from './Section';
import { Button } from '../../../shared/ui/Button';

export function LayoutSection() {
  return (
    <Section title={t('settings.layout')} hint={t('settings.layout.resetHint')}>
      <Button variant="secondary" onClick={resetLayout}>
        {t('settings.layout.reset')}
      </Button>
    </Section>
  );
}
