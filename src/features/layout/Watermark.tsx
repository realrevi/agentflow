import { logoIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { newTerminalConfig } from '../terminal';
import { openPanel, openTerminal } from './layoutStore';

/** Shown by dockview when every panel is closed. */
export function Watermark() {
  return (
    <div class="af-watermark">
      <PixelIcon art={logoIcon} scale={6} />
      <h2 class="af-pixel-title">{t('panel.empty.title')}</h2>
      <p>{t('panel.empty.hint')}</p>
      <div class="af-button-row">
        <button
          type="button"
          class="af-button"
          onClick={() => openTerminal(newTerminalConfig('shell'))}
        >
          {t('sidebar.agents.newShell')}
        </button>
        <button type="button" class="af-button secondary" onClick={() => openPanel('workspace')}>
          {t('panel.add.workspace')}
        </button>
      </div>
    </div>
  );
}
