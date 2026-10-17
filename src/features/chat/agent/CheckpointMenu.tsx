// Header menu: the turns that changed files (time, file count, undone ones marked);
// picking one undoes it after a confirmation (Checkpoints.undo).
import { createSignal, Show } from 'solid-js';
import { Menu } from '../../../shared/ui/Menu';
import { t } from '../../i18n';
import type { Checkpoints } from './checkpoints';

export function CheckpointMenu(props: { checkpoints: Checkpoints; disabled: boolean }) {
  const [anchor, setAnchor] = createSignal<{ x: number; y: number } | null>(null);
  const items = () => {
    const list = props.checkpoints.list();
    if (list.length === 0) return [{ id: '', label: t('agent.checkpoint.none'), disabled: true }];
    return list.map((checkpoint) => {
      const time = new Date(checkpoint.at).toLocaleString();
      return {
        id: checkpoint.turn,
        disabled: checkpoint.revertedAt !== null,
        label:
          checkpoint.revertedAt !== null
            ? t('agent.checkpoint.undone', { time })
            : t('agent.checkpoint.item', { time, count: checkpoint.files.length }),
      };
    });
  };
  return (
    <>
      <button
        type="button"
        class="af-chat-system-toggle"
        disabled={props.disabled}
        aria-haspopup="menu"
        onClick={(event) => {
          const rect = event.currentTarget.getBoundingClientRect();
          void props.checkpoints.refresh().then(() => setAnchor({ x: rect.left, y: rect.bottom }));
        }}
      >
        {t('agent.checkpoints')}
      </button>
      <Show when={anchor()}>
        {(at) => (
          <Menu
            anchor={at()}
            placement="below-start"
            items={items()}
            onSelect={(turn) => void props.checkpoints.undo(turn)}
            onClose={() => setAnchor(null)}
          />
        )}
      </Show>
    </>
  );
}
