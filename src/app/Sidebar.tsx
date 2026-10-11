import { Match, Switch } from 'solid-js';
import { t, type I18nKey } from '../features/i18n';
import { settings, updateSettings } from '../features/settings';
import { charactersIcon, chevronLeftIcon, explorerIcon } from '../shared/ui/icons';
import { PixelIcon } from '../shared/ui/PixelIcon';
import { SidebarSash } from './SidebarSash';
import { AgentsView } from './views/AgentsView';
import { ComingSoonView } from './views/ComingSoonView';

const TITLES: Record<string, I18nKey> = {
  agents: 'activity.agents',
  explorer: 'activity.explorer',
  characters: 'activity.characters',
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
            <ComingSoonView icon={explorerIcon} text={t('sidebar.explorer.soon')} />
          </Match>
          <Match when={settings.ui.activeView === 'characters'}>
            <ComingSoonView icon={charactersIcon} text={t('sidebar.characters.soon')} />
          </Match>
        </Switch>
      </div>
      <SidebarSash />
    </aside>
  );
}
