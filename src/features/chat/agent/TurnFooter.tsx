// The end of a turn that changed files: the files (click opens one) and "Undo this turn",
// or "Undone" once it was restored.
import { For, Show } from 'solid-js';
import type { CheckpointSummary } from '../../../shared/types';
import { Icon } from '../../../shared/ui/Icon';
import { t } from '../../i18n';
import { Button } from '../../../shared/ui/Button';

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
                <Button variant="ghost" size="sm" onClick={() => props.onOpen(path)}>
                  {path}
                </Button>
              </li>
            )}
          </For>
        </ul>
      </details>
      <Show when={!undone()} fallback={<span class="af-badge">{t('agent.turn.undone')}</span>}>
        <Button
          variant="secondary"
          size="sm"
          disabled={props.undoing}
          onClick={() => props.onUndo()}
        >
          <Icon name="undo-2" size={14} />
          {t('agent.turn.undo')}
        </Button>
      </Show>
    </div>
  );
}
