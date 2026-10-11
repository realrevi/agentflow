import { onCleanup, onMount } from 'solid-js';
import { AppShell } from './app/AppShell';
import { Toasts } from './app/Toasts';
import { openTerminal } from './features/layout';
import { installNewShellShortcut, newTerminalConfig } from './features/terminal';

export default function App() {
  onMount(() => {
    const uninstall = installNewShellShortcut(() => openTerminal(newTerminalConfig('shell')));
    onCleanup(uninstall);
  });

  return (
    <>
      <AppShell />
      <Toasts />
    </>
  );
}
