import { For, Show } from 'solid-js';
import { ChatList } from './ChatList';
import { Avatar, spriteFor } from '../../features/characters';
import { t } from '../../features/i18n';
import {
  focusPanel,
  openPanel,
  openPanels,
  openTerminal,
  openTimeline,
} from '../../features/layout';
import { settings } from '../../features/settings';
import { agentState, attentionOf, glowing } from '../../features/agentui';
import { newTerminalConfig, terminalStatus } from '../../features/terminal';
import { openNewWorkstream, WorkstreamsSection } from '../../features/workstreams';

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
                  classList={{ active: panel.active, 'af-glow-item': glowing(panel.id) }}
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
                        state={spriteFor(agentState(panel.id))}
                        scale={1}
                        speed={settings.characters.speed}
                      />
                    )}
                  </Show>
                  {panel.title}
                  <Show when={attentionOf(panel.id)}>
                    {(reason) => (
                      <span
                        class="af-attention-dot"
                        role="img"
                        aria-label={t(`attention.${reason()}`)}
                        title={t(`attention.${reason()}`)}
                      />
                    )}
                  </Show>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <WorkstreamsSection
        panel={(id) => openPanels().find((panel) => panel.id === id)}
        onFocus={focusPanel}
        onTimeline={(id) => openTimeline(id)}
        onNew={() => openNewWorkstream()}
      />
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
