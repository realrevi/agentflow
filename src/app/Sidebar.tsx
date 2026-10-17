import { Match, Switch } from 'solid-js';
import { t, type I18nKey } from '../features/i18n';
import { openPanel, openPanels } from '../features/layout';
import { settings, updateSettings } from '../features/settings';
import { UsageView } from '../features/tokens/UsageView';
import { chevronLeftIcon } from '../shared/ui/icons';
import { PixelIcon } from '../shared/ui/PixelIcon';
import { SidebarSash } from './SidebarSash';
import { AgentsView } from './views/AgentsView';
import { CharactersView } from './views/CharactersView';
import { ExplorerView } from './views/ExplorerView';

const TITLES: Record<string, I18nKey> = {
  agents: 'activity.agents',
  explorer: 'activity.explorer',
  characters: 'activity.characters',
  usage: 'activity.usage',
};

export function Sidebar() {
  return (
    <aside class="af-sidebar" style={{ width: `${settings.ui.sidebarWidth}px` }}>
      <header class="af-sidebar-header">
        <h1 class="af-sidebar-title">{t(TITLES[settings.ui.activeView] ?? 'activity.agents')}</h1>
        <button
          type="button"
          class="af-icon-btn"
          title={t('sidebar.hide')}
          aria-label={t('sidebar.hide')}
          onClick={() => updateSettings((s) => (s.ui.sidebarVisible = false))}
        >
          <PixelIcon art={chevronLeftIcon} />
        </button>
      </header>
      <div class="af-sidebar-body">
        <Switch fallback={<AgentsView />}>
          <Match when={settings.ui.activeView === 'explorer'}>
            <ExplorerView />
          </Match>
          <Match when={settings.ui.activeView === 'characters'}>
            <CharactersView />
          </Match>
          <Match when={settings.ui.activeView === 'usage'}>
            <UsageView
              panelTitle={(id) => openPanels().find((panel) => panel.id === id)?.title}
              onOpenPanel={() => openPanel('usage')}
              onOpenSettings={() => openPanel('settings')}
            />
          </Match>
        </Switch>
      </div>
      <SidebarSash />
    </aside>
  );
}
