// The prompts sent from this panel, newest first. Click to put one back in the prompt bar.
import { For, onCleanup, onMount, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import { showToast } from '../../../shared/toast';
import type { PromptEntry } from '../../../shared/types';
import { copyIcon } from '../../../shared/ui/icons';
import { PixelIcon } from '../../../shared/ui/PixelIcon';
import { locale, reportError, t } from '../../i18n';

interface HistoryPopoverProps {
  entries: readonly PromptEntry[];
  anchor: HTMLElement;
  onPick: (text: string) => void;
  onClose: () => void;
}

export function HistoryPopover(props: HistoryPopoverProps) {
  let box: HTMLDivElement | undefined;
  const time = (at: number) =>
    new Intl.DateTimeFormat(locale(), { dateStyle: 'short', timeStyle: 'short' }).format(at);
  const newestFirst = () => [...props.entries].reverse();

  const onPointerDown = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!box?.contains(target) && !props.anchor.contains(target)) props.onClose();
  };
  const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && props.onClose();
  onMount(() => {
    document.addEventListener('pointerdown', onPointerDown, true);
    document.addEventListener('keydown', onKeyDown);
  });
  onCleanup(() => {
    document.removeEventListener('pointerdown', onPointerDown, true);
    document.removeEventListener('keydown', onKeyDown);
  });

  const copy = async (text: string) => {
    try {
      await ipc.writeClipboard(text);
      showToast({ kind: 'success', title: t('prompt.copied') });
    } catch (err) {
      reportError('error.clipboard', err);
    }
  };

  return (
    <div ref={box} class="af-history" role="dialog" aria-label={t('prompt.history')}>
      <div class="af-history-title">{t('prompt.history')}</div>
      <Show
        when={props.entries.length > 0}
        fallback={<p class="af-history-empty">{t('prompt.historyEmpty')}</p>}
      >
        <ul class="af-history-list">
          <For each={newestFirst()}>
            {(entry) => (
              <li class="af-history-item">
                <button
                  type="button"
                  class="af-history-text"
                  title={entry.text}
                  onClick={() => {
                    props.onPick(entry.text);
                    props.onClose();
                  }}
                >
                  <span class="af-history-time">{time(entry.at)}</span>
                  <span class="af-history-prompt">{entry.text}</span>
                </button>
                <button
                  type="button"
                  class="af-icon-btn af-history-copy"
                  title={t('prompt.copy')}
                  aria-label={t('prompt.copy')}
                  onClick={() => void copy(entry.text)}
                >
                  <PixelIcon art={copyIcon} />
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
