// A small dismissible card with a few first tips, shown after the first-run flow until the
// user closes it.
import { For } from 'solid-js';
import { t, type I18nKey } from '../features/i18n';
import { updateSettings } from '../features/settings';
import { shortcutLabel } from '../shared/platform';
import { IconButton } from '../shared/ui/Button';
import { Icon } from '../shared/ui/Icon';

const TIPS: readonly { key: I18nKey; shortcut?: string }[] = [
  { key: 'tips.add' },
  { key: 'tips.quickOpen', shortcut: 'P' },
  { key: 'tips.sidebar', shortcut: 'B' },
  { key: 'tips.drag' },
];

export function TipsCard() {
  return (
    <aside class="af-tips" aria-label={t('tips.title')}>
      <header class="af-tips-head">
        <Icon name="lightbulb" size={16} />
        <strong>{t('tips.title')}</strong>
        <IconButton
          icon="x"
          size={14}
          label={t('tips.dismiss')}
          onClick={() => updateSettings((s) => (s.onboarding.tipsDismissed = true))}
        />
      </header>
      <ul>
        <For each={TIPS}>
          {(tip) => (
            <li>
              {t(tip.key)}
              {tip.shortcut ? <kbd>{shortcutLabel(tip.shortcut)}</kbd> : null}
            </li>
          )}
        </For>
      </ul>
    </aside>
  );
}
