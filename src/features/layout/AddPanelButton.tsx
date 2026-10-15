import { createSignal, Match, Switch } from 'solid-js';
import type { PtyKind } from '../../shared/types';
import { plusIcon } from '../../shared/ui/icons';
import { InlinePrompt } from '../../shared/ui/InlinePrompt';
import { Menu, type MenuItem } from '../../shared/ui/Menu';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { NewChatPrompt } from '../chat';
import { t } from '../i18n';
import { AGENT_KINDS, cliFor, kindLabel, loadCliStatus, newTerminalConfig } from '../terminal';
import { startChat } from './chatPanels';
import { openPanel, openTerminal } from './layoutStore';

type Mode = 'closed' | 'menu' | 'custom' | 'api';

/** The "+" in every group header: terminals, API agents, and (re)opening Files / Settings. */
export function AddPanelButton(props: { groupId: string }) {
  const [mode, setMode] = createSignal<Mode>('closed');
  let button: HTMLButtonElement | undefined;

  const agentItem = (kind: PtyKind): MenuItem => {
    const cli = cliFor(kind);
    const missing = cli?.found === false;
    return {
      id: kind,
      label: kindLabel(kind),
      disabled: missing,
      title: missing ? t('terminal.notFound') : (cli?.path ?? undefined),
    };
  };

  const items = (): MenuItem[] => [
    { id: 'shell', label: kindLabel('shell'), heading: t('panel.add.terminals') },
    ...AGENT_KINDS.map(agentItem),
    { id: 'custom', label: t('terminal.add.custom') },
    { id: 'api', label: t('panel.add.api') },
    { id: 'files', label: t('panel.add.files'), heading: t('panel.add.panels') },
    { id: 'settings', label: t('panel.add.settings') },
  ];

  const select = (id: string) => {
    if (id === 'custom' || id === 'api') setMode(id);
    else if (id === 'files' || id === 'settings') openPanel(id, props.groupId);
    else if (id === 'shell' || (AGENT_KINDS as readonly string[]).includes(id)) {
      openTerminal(newTerminalConfig(id as PtyKind), props.groupId);
    }
  };

  const toggle = () => {
    if (mode() !== 'closed') {
      setMode('closed');
    } else {
      void loadCliStatus();
      setMode('menu');
    }
  };

  return (
    <>
      <button
        ref={button}
        type="button"
        class="af-icon-btn"
        title={t('panel.add')}
        aria-label={t('panel.add')}
        aria-haspopup="menu"
        aria-expanded={mode() !== 'closed'}
        onClick={toggle}
      >
        <PixelIcon art={plusIcon} />
      </button>
      <Switch>
        <Match when={mode() === 'menu' && button}>
          {(anchor) => (
            <Menu
              anchor={anchor()}
              items={items()}
              placement="below-end"
              onSelect={select}
              // "Custom command…" and "API agent…" switch to their prompt instead of closing.
              onClose={() => setMode((current) => (current === 'menu' ? 'closed' : current))}
            />
          )}
        </Match>
        <Match when={mode() === 'api' && button}>
          {(anchor) => (
            <NewChatPrompt
              anchor={anchor()}
              placement="below-end"
              onStart={(providerId, model) => void startChat(providerId, model, props.groupId)}
              onOpenSettings={() => openPanel('settings')}
              onClose={() => setMode('closed')}
            />
          )}
        </Match>
        <Match when={mode() === 'custom' && button}>
          {(anchor) => (
            <InlinePrompt
              anchor={anchor()}
              placement="below-end"
              label={t('terminal.customPrompt')}
              placeholder={t('terminal.customPlaceholder')}
              submitLabel={t('terminal.run')}
              onSubmit={(command) =>
                openTerminal(newTerminalConfig('custom', command), props.groupId)
              }
              onClose={() => setMode('closed')}
            />
          )}
        </Match>
      </Switch>
    </>
  );
}
