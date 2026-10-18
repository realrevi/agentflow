import { createSignal, Show } from 'solid-js';
import { t } from '../features/i18n';
import {
  currentWorkspace,
  folderName,
  openFolder,
  recentWorkspaces,
  setWorkspace,
} from '../features/workspace';
import { Menu, type MenuItem } from '../shared/ui/Menu';
import { Icon } from '../shared/ui/Icon';

const OPEN_ID = 'open';

/** Status bar item: the workspace folder name; click for recent folders + "Open folder…". */
export function WorkspaceStatus() {
  const [open, setOpen] = createSignal(false);
  let button: HTMLButtonElement | undefined;

  const label = () => {
    const path = currentWorkspace();
    return path ? folderName(path) : t('workspace.none');
  };
  const tooltip = () => {
    const path = currentWorkspace();
    return path ? t('status.workspace', { path }) : t('status.workspaceNone');
  };

  const items = (): MenuItem[] => [
    ...recentWorkspaces().map((path, index) => ({
      id: `recent:${index}`,
      label: folderName(path),
      title: path,
      checked: path === currentWorkspace(),
      ...(index === 0 ? { heading: t('workspace.recent') } : {}),
    })),
    { id: OPEN_ID, label: t('workspace.open') },
  ];

  const select = (id: string) => {
    if (id === OPEN_ID) return void openFolder();
    const path = recentWorkspaces()[Number(id.slice('recent:'.length))];
    if (path) setWorkspace(path);
  };

  return (
    <>
      <button
        ref={button}
        type="button"
        class="af-status-item"
        title={tooltip()}
        aria-haspopup="menu"
        aria-expanded={open()}
        onClick={() => setOpen((value) => !value)}
      >
        <Icon name="folder" size={13} />
        {label()}
      </button>
      <Show when={open() && button}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            items={items()}
            placement="above-start"
            onSelect={select}
            onClose={() => setOpen(false)}
          />
        )}
      </Show>
    </>
  );
}
