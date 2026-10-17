import { onCleanup, onMount } from 'solid-js';
import { AppShell } from './app/AppShell';
import { installQuitGuard } from './app/quitGuard';
import { Dialogs } from './app/Dialogs';
import { Toasts } from './app/Toasts';
import { watchProfiles } from './features/agents';
import { installQuickOpenShortcut, QuickOpen, type PaletteCommand } from './features/files';
import { t } from './features/i18n';
import { openEditor, openPanel, openTerminal } from './features/layout';
import { installNewShellShortcut, newTerminalConfig } from './features/terminal';

/** Commands of the palette's `>` mode. */
const commands = (): PaletteCommand[] => [
  { id: 'installer', label: t('command.openInstaller'), run: () => openPanel('installer') },
  { id: 'settings', label: t('command.openSettings'), run: () => openPanel('settings') },
  { id: 'usage', label: t('command.openUsage'), run: () => openPanel('usage') },
  {
    id: 'shell',
    label: t('command.newShell'),
    run: () => openTerminal(newTerminalConfig('shell')),
  },
];

export default function App() {
  onMount(() => {
    const uninstall = [
      installNewShellShortcut(() => openTerminal(newTerminalConfig('shell'))),
      installQuickOpenShortcut(),
      installQuitGuard(),
      watchProfiles(),
    ];
    onCleanup(() => uninstall.forEach((fn) => fn()));
  });

  return (
    <>
      <AppShell />
      <QuickOpen onOpen={openEditor} commands={commands} />
      <Dialogs />
      <Toasts />
    </>
  );
}
