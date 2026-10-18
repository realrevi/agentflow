// The activity bar: always visible. Views toggle the sidebar (clicking the active one hides
// it, like VS Code); the rest open panels. The pixel logo is the one playful accent here.
import { For, type JSX } from 'solid-js';
import type { SidebarView } from '../shared/types';
import { logoIcon } from '../shared/ui/icons';
import { Icon, type IconName } from '../shared/ui/Icon';
import { PixelIcon } from '../shared/ui/PixelIcon';
import { Tooltip } from '../shared/ui/Surface';
import { t, type I18nKey } from '../features/i18n';
import { openPanel, openStage } from '../features/layout';
import { settings } from '../features/settings';
import { selectView } from './panelToggles';

const VIEWS: readonly { id: SidebarView; icon: IconName; label: I18nKey }[] = [
  { id: 'agents', icon: 'square-terminal', label: 'activity.agents' },
  { id: 'explorer', icon: 'files', label: 'activity.explorer' },
  { id: 'characters', icon: 'face-slightly-smiling', label: 'activity.characters' },
  { id: 'usage', icon: 'chart-column', label: 'activity.usage' },
];

function Activity(props: {
  icon: IconName;
  label: string;
  active?: boolean;
  onClick: () => void;
}): JSX.Element {
  return (
    <Tooltip text={props.label}>
      <button
        type="button"
        class="af-activity"
        classList={{ active: Boolean(props.active) }}
        aria-label={props.label}
        aria-pressed={props.active === undefined ? undefined : props.active}
        onClick={() => props.onClick()}
      >
        <Icon name={props.icon} size={20} strokeWidth={1.75} />
      </button>
    </Tooltip>
  );
}

export function ActivityBar() {
  return (
    <nav class="af-activitybar" aria-label={t('activity.bar')}>
      <div class="af-activitybar-logo" title="AgentFlow">
        <PixelIcon art={logoIcon} />
      </div>
      <For each={VIEWS}>
        {(view) => (
          <Activity
            icon={view.icon}
            label={t(view.label)}
            active={settings.ui.sidebarVisible && settings.ui.activeView === view.id}
            onClick={() => selectView(view.id)}
          />
        )}
      </For>
      <Activity
        icon="library-big"
        label={t('activity.library')}
        onClick={() => openPanel('installer')}
      />
      <Activity icon="drama" label={t('activity.stage')} onClick={() => openStage()} />
      <div class="af-activitybar-spacer" />
      <Activity
        icon="settings"
        label={t('activity.settings')}
        onClick={() => openPanel('settings')}
      />
    </nav>
  );
}
