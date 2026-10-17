// Shown by dockview in empty groups and when the whole dock is empty. The center (agent
// area) offers the agent CLIs; they never start by themselves.
import { createSignal, For, onMount, Show } from 'solid-js';
import { shortcutLabel } from '../../shared/platform';
import type { PtyKind } from '../../shared/types';
import { agentsIcon, logoIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { NewChatPrompt } from '../chat';
import { t } from '../i18n';
import { AGENT_KINDS, cliFor, kindLabel, loadCliStatus, newTerminalConfig } from '../terminal';
import { agentProfiles } from '../agents';
import { openProfileChat, startChat } from './chatPanels';
import { openPanel, openTerminal } from './layoutStore';
import { groupRole } from './roles';

function AgentArea(props: { groupId: string }) {
  onMount(() => void loadCliStatus());
  const start = (kind: PtyKind) => openTerminal(newTerminalConfig(kind), props.groupId);
  const [asking, setAsking] = createSignal(false);
  let apiButton: HTMLButtonElement | undefined;
  return (
    <div class="af-watermark">
      <PixelIcon art={agentsIcon} scale={5} />
      <h2 class="af-pixel-title">{t('agents.area.title')}</h2>
      <p>{t('agents.area.hint')}</p>
      <div class="af-button-row">
        <For each={AGENT_KINDS}>
          {(kind) => {
            const missing = () => cliFor(kind)?.found === false;
            return (
              <button
                type="button"
                class="af-button"
                disabled={missing()}
                title={missing() ? t('terminal.notFound') : undefined}
                onClick={() => start(kind)}
              >
                {kindLabel(kind)}
              </button>
            );
          }}
        </For>
        <button
          ref={apiButton}
          type="button"
          class="af-button secondary"
          aria-haspopup="dialog"
          aria-expanded={asking()}
          onClick={() => setAsking((open) => !open)}
        >
          {t('panel.add.api')}
        </button>
      </div>
      <Show when={agentProfiles().length > 0}>
        <p class="af-watermark-hint">{t('agents.area.installed')}</p>
        <div class="af-button-row">
          <For each={agentProfiles()}>
            {(profile) => (
              <button
                type="button"
                class="af-button secondary"
                title={profile.description}
                onClick={() => void openProfileChat(profile, props.groupId)}
              >
                {profile.name}
              </button>
            )}
          </For>
        </div>
      </Show>
      <Show when={asking() && apiButton}>
        {(anchor) => (
          <NewChatPrompt
            anchor={anchor()}
            placement="below-start"
            onStart={(providerId, model) => void startChat(providerId, model, props.groupId)}
            onOpenSettings={() => openPanel('settings')}
            onClose={() => setAsking(false)}
          />
        )}
      </Show>
      <p class="af-watermark-hint">
        {t('agents.area.quickOpen', { shortcut: shortcutLabel('P') })}
      </p>
    </div>
  );
}

function EmptyDock() {
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
        <button type="button" class="af-button secondary" onClick={() => openPanel('files')}>
          {t('sidebar.agents.showFiles')}
        </button>
      </div>
    </div>
  );
}

export function Watermark(props: { groupId?: string }) {
  const center = () =>
    props.groupId && groupRole(props.groupId) === 'agents' ? props.groupId : null;
  return (
    <Show when={center()} fallback={<EmptyDock />}>
      {(groupId) => <AgentArea groupId={groupId()} />}
    </Show>
  );
}
