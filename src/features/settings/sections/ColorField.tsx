import { Show } from 'solid-js';
import { toHex6 } from '../../../shared/color';
import { t } from '../../i18n';

interface ColorFieldProps {
  id: string;
  /** The user's override, or `null` when following the theme. */
  value: string | null;
  /** What is shown while there is no override (the theme's current color). */
  fallback: string;
  onChange: (color: string | null) => void;
  resetLabel?: string;
}

/** Native color picker plus a reset button; updates live while dragging. */
export function ColorField(props: ColorFieldProps) {
  const shown = () => toHex6(props.value ?? props.fallback) ?? '#000000';
  return (
    <div class="af-color-field">
      <input
        id={props.id}
        type="color"
        value={shown()}
        onInput={(event) => props.onChange(event.currentTarget.value)}
      />
      <code class="af-color-value">{shown()}</code>
      <Show when={props.value !== null}>
        <button
          type="button"
          class="af-button secondary small"
          onClick={() => props.onChange(null)}
        >
          {props.resetLabel ?? t('settings.useTheme')}
        </button>
      </Show>
    </div>
  );
}
