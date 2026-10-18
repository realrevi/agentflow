import { lazy, onCleanup, onMount, Show, Suspense } from 'solid-js';
import { AppShell } from './app/AppShell';
import { installQuitGuard } from './app/quitGuard';
import { Dialogs } from './app/Dialogs';
import { Toasts } from './app/Toasts';
import { watchProfiles } from './features/agents';
import { installQuickOpenShortcut, QuickOpen, type PaletteCommand } from './features/files';
import { t } from './features/i18n';
import {
  focusPanel,
  openEditor,
  openPanel,
  openPanels,
  openStage,
  openTerminal,
  openTimeline,
} from './features/layout';
import { settings } from './features/settings';
import { installNewShellShortcut, newTerminalConfig } from './features/terminal';
import {
  HandoffCards,
  installWorkstreams,
  openNewWorkstream,
  stopAll,
  workstreams,
} from './features/workstreams';
import { NewWorkstreamDialog } from './app/workstreams/NewWorkstreamDialog';
import { installToggleShortcuts } from './app/panelToggles';
import { TipsCard } from './app/TipsCard';

// Loaded only while the first-run flow is not done.
const Onboarding = lazy(() => import('./app/onboarding/Onboarding'));

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
  { id: 'workstream', label: t('command.newWorkstream'), run: () => openNewWorkstream() },
  { id: 'stage', label: t('command.openStage'), run: () => openStage() },
  {
    id: 'timeline',
    label: t('command.openTimeline'),
    run: () => openTimeline(workstreams()[0]?.id ?? null),
  },
  { id: 'stopAll', label: t('command.stopWorkstreams'), run: stopAll },
];

const panelTitle = (id: string) => openPanels().find((panel) => panel.id === id)?.title ?? id;

export default function App() {
  onMount(() => {
    const uninstall = [
      installNewShellShortcut(() => openTerminal(newTerminalConfig('shell'))),
      installQuickOpenShortcut(),
      installToggleShortcuts(),
      installQuitGuard(),
      watchProfiles(),
      installWorkstreams({
        panelTitle,
        isWatched: (id) =>
          document.hasFocus() && openPanels().some((panel) => panel.id === id && panel.active),
        workspace: () => settings.currentWorkspace,
      }),
    ];
    onCleanup(() => uninstall.forEach((fn) => fn()));
  });

  return (
    <>
      <AppShell />
      <QuickOpen onOpen={openEditor} commands={commands} />
      <HandoffCards onFocus={focusPanel} />
      <NewWorkstreamDialog />
      <Show when={settings.onboarding.done && !settings.onboarding.tipsDismissed}>
        <TipsCard />
      </Show>
      <Show when={!settings.onboarding.done}>
        <Suspense>
          <Onboarding />
        </Suspense>
      </Show>
      <Dialogs />
      <Toasts />
    </>
  );
}
