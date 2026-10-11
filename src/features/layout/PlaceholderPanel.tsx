import { Show } from 'solid-js';
import { agentsIcon, explorerIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { panelTitle, type PanelParams } from './panels';

/** Phase 1 stand-in for agent terminals (phase 2) and the workspace (phase 3). */
export function PlaceholderPanel(props: { params: PanelParams }) {
  const isAgent = () => props.params.kind === 'agent';
  return (
    <div class="af-placeholder">
      <PixelIcon art={isAgent() ? agentsIcon : explorerIcon} scale={4} />
      <Show when={isAgent()} fallback={<h2 class="af-pixel-title">{panelTitle(props.params)}</h2>}>
        <h2 class="af-pixel-title af-agent-bold">{panelTitle(props.params)}</h2>
      </Show>
      <p>{isAgent() ? t('panel.agent.body') : t('panel.workspace.body')}</p>
    </div>
  );
}
