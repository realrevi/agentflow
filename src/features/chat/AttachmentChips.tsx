// Files waiting to be sent with the next prompt, with a rough token count each. A file this
// chat already sent is marked "sent earlier": requests carry only its newest copy.
import { For, Show } from 'solid-js';
import type { Attachment } from '../../shared/types';
import { closeIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { estimateTokens } from './attachments';

export function AttachmentChips(props: {
  files: readonly Attachment[];
  onRemove: (path: string) => void;
  /** The chat sent this path before (the older copy becomes a short reference). */
  sentBefore?: (path: string) => boolean;
}) {
  return (
    <Show when={props.files.length > 0}>
      <div class="af-chips af-chat-attachments">
        <For each={props.files}>
          {(file) => {
            const name = () => file.path.split('/').pop() ?? file.path;
            return (
              <span
                class="af-chip"
                classList={{ 'sent-before': props.sentBefore?.(file.path) ?? false }}
                title={
                  props.sentBefore?.(file.path)
                    ? t('chat.attach.resend', { path: file.path })
                    : file.path
                }
              >
                {name()}
                <span class="af-chip-tokens">
                  {t('chat.attach.tokens', { count: estimateTokens(file.content) })}
                </span>
                <Show when={props.sentBefore?.(file.path)}>
                  <span class="af-chip-note">{t('chat.attach.sentEarlier')}</span>
                </Show>
                <button
                  type="button"
                  class="af-chip-remove"
                  title={t('chat.attach.remove', { name: name() })}
                  aria-label={t('chat.attach.remove', { name: name() })}
                  onClick={() => props.onRemove(file.path)}
                >
                  <PixelIcon art={closeIcon} />
                </button>
              </span>
            );
          }}
        </For>
      </div>
    </Show>
  );
}
