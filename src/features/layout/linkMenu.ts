// The tab's context menu for agent panels: link the panel to a workstream as a worker (named
// after its character, unique in the workstream), start a new workstream with it as the
// manager, or unlink it.
import type { MenuItem } from '../../shared/ui/Menu';
import { t } from '../i18n';
import { openNewWorkstream } from '../workstreams/dialogState';
import { linkWorker, unlinkPanel, workstreamOf, workstreams } from '../workstreams/store';
import { panelCharacterName } from './stagePanels';

export function linkMenuItems(panelId: string): MenuItem[] {
  const own = workstreamOf(panelId);
  if (own) {
    return [
      {
        id: 'unlink',
        label:
          own.manager === panelId
            ? t('workstream.link.endAsManager', { name: own.name })
            : t('workstream.link.unlink', { name: own.name }),
      },
    ];
  }
  return [
    ...workstreams().map((ws, index) => ({
      id: `link:${ws.id}`,
      label: ws.name,
      heading: index === 0 ? t('workstream.link.heading') : undefined,
    })),
    { id: 'new', label: t('workstream.link.newAsManager') },
  ];
}

export async function selectLinkItem(panelId: string, id: string, title: string): Promise<void> {
  if (id === 'unlink') {
    unlinkPanel(panelId);
    return;
  }
  if (id === 'new') {
    openNewWorkstream({ manager: panelId });
    return;
  }
  const wsId = id.startsWith('link:') ? id.slice(5) : null;
  const ws = workstreams().find((item) => item.id === wsId);
  if (!ws) return;
  const base = (await panelCharacterName(panelId)) ?? title;
  const taken = new Set(Object.values(ws.names).map((name) => name.toLowerCase()));
  let name = base;
  for (let n = 2; taken.has(name.toLowerCase()); n++) name = `${base} ${n}`;
  linkWorker(ws.id, panelId, name);
}
