// The conversation's permission mode as a compact segmented control with icons; each
// segment's tooltip says what the mode allows.
import { For } from 'solid-js';
import type { PermissionMode } from '../../../shared/types';
import { PixelIcon, type PixelArt } from '../../../shared/ui/PixelIcon';
import { t, type I18nKey } from '../../i18n';
import { editIcon, eyeIcon, shieldIcon } from './toolIcons';

const MODES: readonly { mode: PermissionMode; icon: PixelArt; label: I18nKey; hint: I18nKey }[] = [
  { mode: 'ask', icon: shieldIcon, label: 'agent.mode.ask', hint: 'agent.mode.askHint' },
  {
    mode: 'autoEdit',
    icon: editIcon,
    label: 'agent.mode.autoEdit',
    hint: 'agent.mode.autoEditHint',
  },
  {
    mode: 'readOnly',
    icon: eyeIcon,
    label: 'agent.mode.readOnly',
    hint: 'agent.mode.readOnlyHint',
  },
];

export function ModePill(props: {
  mode: PermissionMode;
  onChange: (mode: PermissionMode) => void;
}) {
  return (
    <div class="af-mode-pill" role="radiogroup" aria-label={t('agent.mode')}>
      <For each={MODES}>
        {(item) => (
          <button
            type="button"
            role="radio"
            aria-checked={props.mode === item.mode}
            classList={{ active: props.mode === item.mode }}
            title={t(item.hint)}
            onClick={() => props.mode !== item.mode && props.onChange(item.mode)}
          >
            <PixelIcon art={item.icon} />
            <span>{t(item.label)}</span>
          </button>
        )}
      </For>
    </div>
  );
}
