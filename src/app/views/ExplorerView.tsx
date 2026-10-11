import { For, Show } from 'solid-js';
import { openQuickOpen } from '../../features/files';
import { t } from '../../features/i18n';
import { openPanel } from '../../features/layout';
import {
  currentWorkspace,
  folderName,
  openFolder,
  recentWorkspaces,
  setWorkspace,
} from '../../features/workspace';
import { shortcutLabel } from '../../shared/platform';

/** The workspace folder at a glance; the tree itself lives in the Files dock panel. */
export function ExplorerView() {
  const others = () => recentWorkspaces().filter((path) => path !== currentWorkspace());
  return (
    <div class="af-view">
      <h2 class="af-view-heading">{t('sidebar.explorer.workspace')}</h2>
      <Show
        when={currentWorkspace()}
        fallback={<p class="af-view-empty">{t('files.noWorkspace')}</p>}
      >
        {(root) => (
          <p class="af-view-folder" title={root()}>
            <strong>{folderName(root())}</strong>
            <span>{root()}</span>
          </p>
        )}
      </Show>
      <div class="af-button-column">
        <Show when={currentWorkspace()}>
          <button type="button" class="af-button" onClick={() => openPanel('files')}>
            {t('sidebar.agents.showFiles')}
          </button>
          <button type="button" class="af-button secondary" onClick={openQuickOpen}>
            {t('sidebar.explorer.quickOpen', { shortcut: shortcutLabel('P') })}
          </button>
        </Show>
        <button type="button" class="af-button secondary" onClick={() => void openFolder()}>
          {t('workspace.open')}
        </button>
      </div>
      <Show when={others().length > 0}>
        <h2 class="af-view-heading af-view-gap">{t('workspace.recent')}</h2>
        <ul class="af-list">
          <For each={others()}>
            {(path) => (
              <li>
                <button
                  type="button"
                  class="af-list-item"
                  title={path}
                  onClick={() => setWorkspace(path)}
                >
                  {folderName(path)}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
