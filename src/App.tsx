import { onCleanup, onMount } from 'solid-js';
import { AppShell } from './app/AppShell';
import { Dialogs } from './app/Dialogs';
import { Toasts } from './app/Toasts';
import { installQuickOpenShortcut, QuickOpen } from './features/files';
import { openEditor, openTerminal } from './features/layout';
import { installNewShellShortcut, newTerminalConfig } from './features/terminal';

export default function App() {
  onMount(() => {
    const uninstall = [
      installNewShellShortcut(() => openTerminal(newTerminalConfig('shell'))),
      installQuickOpenShortcut(),
    ];
    onCleanup(() => uninstall.forEach((fn) => fn()));
  });

  return (
    <>
      <AppShell />
      <QuickOpen onOpen={openEditor} />
      <Dialogs />
      <Toasts />
    </>
  );
}
