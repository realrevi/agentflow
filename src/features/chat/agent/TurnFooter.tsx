// The end of a turn that changed files: the files (click opens one) and "Undo this turn",
// or "Undone" once it was restored.
import { For, Show } from 'solid-js';
import type { CheckpointSummary } from '../../../shared/types';
import { PixelIcon } from '../../../shared/ui/PixelIcon';
import { t } from '../../i18n';
import { undoIcon } from './toolIcons';

export function TurnFooter(props: {
  checkpoint: CheckpointSummary;
  undoing: boolean;
  onOpen: (path: string) => void;
  onUndo: () => void;
}) {
  const undone = () => props.checkpoint.revertedAt !== null;
  return (
    <div class="af-turn-footer" classList={{ undone: undone() }}>
      <details class="af-turn-files">
        <summary>{t('agent.turn.changed', { count: props.checkpoint.files.length })}</summary>
        <ul>
          <For each={props.checkpoint.files}>
            {(path) => (
              <li>
                <button type="button" class="af-link-btn" onClick={() => props.onOpen(path)}>
                  {path}
                </button>
              </li>
            )}
          </For>
        </ul>
      </details>
      <Show when={!undone()} fallback={<span class="af-badge">{t('agent.turn.undone')}</span>}>
        <button
          type="button"
          class="af-button secondary small"
          disabled={props.undoing}
          onClick={() => props.onUndo()}
        >
          <PixelIcon art={undoIcon} />
          {t('agent.turn.undo')}
        </button>
      </Show>
    </div>
  );
}
