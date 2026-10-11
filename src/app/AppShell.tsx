import { Show } from 'solid-js';
import { DockviewHost } from '../features/layout';
import { settings } from '../features/settings';
import { ActivityBar } from './ActivityBar';
import { Sidebar } from './Sidebar';
import { StatusBar } from './StatusBar';

/** VS Code-style frame: activity bar | sidebar | dock, with a status bar underneath. */
export function AppShell() {
  return (
    <div class="af-shell">
      <div class="af-shell-body">
        <ActivityBar />
        <Show when={settings.ui.sidebarVisible}>
          <Sidebar />
        </Show>
        <main class="af-main">
          <DockviewHost />
        </main>
      </div>
      <StatusBar />
    </div>
  );
}
