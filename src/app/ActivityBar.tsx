import { For } from 'solid-js';
import type { SidebarView } from '../shared/types';
import {
  agentsIcon,
  charactersIcon,
  explorerIcon,
  libraryIcon,
  logoIcon,
  settingsIcon,
  usageIcon,
} from '../shared/ui/icons';
import { PixelIcon, type PixelArt } from '../shared/ui/PixelIcon';
import { t, type I18nKey } from '../features/i18n';
import { openPanel } from '../features/layout';
import { settings, updateSettings } from '../features/settings';

const VIEWS: readonly { id: SidebarView; icon: PixelArt; label: I18nKey }[] = [
  { id: 'agents', icon: agentsIcon, label: 'activity.agents' },
  { id: 'explorer', icon: explorerIcon, label: 'activity.explorer' },
  { id: 'characters', icon: charactersIcon, label: 'activity.characters' },
  { id: 'usage', icon: usageIcon, label: 'activity.usage' },
];

/** Clicking the active view toggles the sidebar, like VS Code. */
function selectView(view: SidebarView): void {
  updateSettings((s) => {
    s.ui.sidebarVisible = !(s.ui.sidebarVisible && s.ui.activeView === view);
    s.ui.activeView = view;
  });
}

export function ActivityBar() {
  return (
    <nav class="af-activitybar" aria-label={t('activity.bar')}>
      <div class="af-activitybar-logo" title="AgentFlow">
        <PixelIcon art={logoIcon} />
      </div>
      <For each={VIEWS}>
        {(view) => {
          const active = () => settings.ui.sidebarVisible && settings.ui.activeView === view.id;
          return (
            <button
              type="button"
              class="af-activity"
              classList={{ active: active() }}
              title={t(view.label)}
              aria-label={t(view.label)}
              aria-pressed={active()}
              onClick={() => selectView(view.id)}
            >
              <PixelIcon art={view.icon} />
            </button>
          );
        }}
      </For>
      <button
        type="button"
        class="af-activity"
        title={t('activity.library')}
        aria-label={t('activity.library')}
        onClick={() => openPanel('installer')}
      >
        <PixelIcon art={libraryIcon} />
      </button>
      <div class="af-activitybar-spacer" />
      <button
        type="button"
        class="af-activity"
        title={t('activity.settings')}
        aria-label={t('activity.settings')}
        onClick={() => openPanel('settings')}
      >
        <PixelIcon art={settingsIcon} />
      </button>
    </nav>
  );
}
