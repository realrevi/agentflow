import { For, Show } from 'solid-js';
import { t } from '../features/i18n';
import { dismissToast, toasts } from '../shared/toast';
import { closeIcon } from '../shared/ui/icons';
import { PixelIcon } from '../shared/ui/PixelIcon';

export function Toasts() {
  return (
    <div class="af-toasts" role="status" aria-live="polite">
      <For each={toasts()}>
        {(toast) => (
          <div class={`af-toast af-toast-${toast.kind}`}>
            <div class="af-toast-text">
              <strong>{toast.title}</strong>
              <Show when={toast.detail}>
                <span class="af-toast-detail">{toast.detail}</span>
              </Show>
            </div>
            <button
              type="button"
              class="af-icon-btn"
              title={t('toast.dismiss')}
              aria-label={t('toast.dismiss')}
              onClick={() => dismissToast(toast.id)}
            >
              <PixelIcon art={closeIcon} />
            </button>
          </div>
        )}
      </For>
    </div>
  );
}
