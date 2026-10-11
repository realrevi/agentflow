import { explorerIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';

/** Phase 2 stand-in for the workspace (file tree + editor arrive in phase 3). */
export function PlaceholderPanel() {
  return (
    <div class="af-placeholder">
      <PixelIcon art={explorerIcon} scale={4} />
      <h2 class="af-pixel-title">{t('panel.workspace.title')}</h2>
      <p>{t('panel.workspace.body')}</p>
    </div>
  );
}
