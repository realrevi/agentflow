import { createSignal, Match, Switch } from 'solid-js';
import type { PtyKind } from '../../shared/types';
import { plusIcon } from '../../shared/ui/icons';
import { InlinePrompt } from '../../shared/ui/InlinePrompt';
import { Menu, type MenuItem } from '../../shared/ui/Menu';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { AGENT_KINDS, cliFor, kindLabel, loadCliStatus, newTerminalConfig } from '../terminal';
import { openPanel, openTerminal } from './layoutStore';

type Mode = 'closed' | 'menu' | 'custom';

/** The "+" in every group header: new terminals, plus (re)opening Workspace / Settings. */
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
    { id: 'workspace', label: t('panel.add.workspace'), heading: t('panel.add.panels') },
    { id: 'settings', label: t('panel.add.settings') },
  ];

  const select = (id: string) => {
    if (id === 'custom') setMode('custom');
    else if (id === 'workspace' || id === 'settings') openPanel(id, props.groupId);
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
              // Choosing "Custom command…" switches to the prompt instead of closing.
              onClose={() => setMode((current) => (current === 'menu' ? 'closed' : current))}
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
