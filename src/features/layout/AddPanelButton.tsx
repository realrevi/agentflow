import { createSignal, Show } from 'solid-js';
import { plusIcon } from '../../shared/ui/icons';
import { Menu, type MenuItem } from '../../shared/ui/Menu';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { openPanel } from './layoutStore';
import { isPanelKind } from './panels';

/** The "+" in every group header: add an agent, or (re)open Workspace / Settings. */
export function AddPanelButton(props: { groupId: string }) {
  const [open, setOpen] = createSignal(false);
  let button: HTMLButtonElement | undefined;

  const items = (): MenuItem[] => [
    { id: 'agent', label: t('panel.add.agent') },
    { id: 'workspace', label: t('panel.add.workspace') },
    { id: 'settings', label: t('panel.add.settings') },
  ];

  return (
    <>
      <button
        ref={button}
        type="button"
        class="af-icon-btn"
        title={t('panel.add')}
        aria-label={t('panel.add')}
        aria-haspopup="menu"
        aria-expanded={open()}
        onClick={() => setOpen((value) => !value)}
      >
        <PixelIcon art={plusIcon} />
      </button>
      <Show when={open() && button}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            items={items()}
            placement="below-end"
            onSelect={(id) => isPanelKind(id) && openPanel(id, props.groupId)}
            onClose={() => setOpen(false)}
          />
        )}
      </Show>
    </>
  );
}
