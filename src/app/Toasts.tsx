import { For, Show } from 'solid-js';
import { t } from '../features/i18n';
import { dismissToast, toasts } from '../shared/toast';
import { Button, IconButton } from '../shared/ui/Button';

export function Toasts() {
  return (
    <div class="af-toasts" role="status" aria-live="polite">
      <For each={toasts()}>
        {(toast) => (
          <div class={`af-toast af-toast-${toast.kind}`} classList={{ leaving: toast.leaving }}>
            <div class="af-toast-text">
              <strong>{toast.title}</strong>
              <Show when={toast.detail}>
                <span class="af-toast-detail">{toast.detail}</span>
              </Show>
            </div>
            <Show when={toast.action}>
              {(action) => (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    dismissToast(toast.id);
                    action().run();
                  }}
                >
                  {action().label}
                </Button>
              )}
            </Show>
            <IconButton
              icon="x"
              label={t('toast.dismiss')}
              onClick={() => dismissToast(toast.id)}
            />
          </div>
        )}
      </For>
    </div>
  );
}
