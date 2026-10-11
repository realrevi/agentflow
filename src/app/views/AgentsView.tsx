import { For, Show } from 'solid-js';
import { t } from '../../features/i18n';
import { focusPanel, openPanel, openPanels } from '../../features/layout';

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
                  classList={{
                    active: panel.active,
                    'af-agent-bold': panel.kind === 'agent' && panel.active,
                  }}
                  onClick={() => focusPanel(panel.id)}
                >
                  <span class={`af-dot af-dot-${panel.kind}`} aria-hidden="true" />
                  {panel.title}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <div class="af-button-column">
        <button type="button" class="af-button" onClick={() => openPanel('agent')}>
          {t('sidebar.agents.newAgent')}
        </button>
        <button type="button" class="af-button secondary" onClick={() => openPanel('workspace')}>
          {t('sidebar.agents.openWorkspace')}
        </button>
      </div>
    </div>
  );
}
