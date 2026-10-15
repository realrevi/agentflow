import { For, Show } from 'solid-js';
import { ChatList } from './ChatList';
import { Avatar } from '../../features/characters';
import { t } from '../../features/i18n';
import { focusPanel, openPanel, openPanels, openTerminal } from '../../features/layout';
import { settings } from '../../features/settings';
import { agentState } from '../../features/agentui';
import { newTerminalConfig, terminalStatus } from '../../features/terminal';

export function AgentsView() {
  return (
    <div class="af-view">
      <h2 class="af-view-heading">{t('sidebar.agents.panels')}</h2>
      <Show
        when={openPanels().length > 0}
        fallback={<p class="af-view-empty">{t('sidebar.agents.none')}</p>}
      >
        <ul class="af-list">
          <For each={openPanels()}>
            {(panel) => (
              <li>
                <button
                  type="button"
                  class="af-list-item"
                  classList={{ active: panel.active }}
                  onClick={() => focusPanel(panel.id)}
                >
                  <Show
                    when={panel.character}
                    fallback={
                      <span
                        class={`af-dot af-dot-${panel.kind}`}
                        classList={{ running: terminalStatus(panel.id) === 'running' }}
                        aria-hidden="true"
                      />
                    }
                  >
                    {(character) => (
                      <Avatar
                        character={character()}
                        state={agentState(panel.id)}
                        scale={1}
                        speed={settings.characters.speed}
                      />
                    )}
                  </Show>
                  {panel.title}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <ChatList />
      <div class="af-button-column">
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
