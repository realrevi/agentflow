// Tab for terminal panels: dockview's default tab look plus a running/exited status dot.
import { createCloseButton, type ITabRenderer } from 'dockview-core';
import { createSignal, onCleanup } from 'solid-js';
import type { DockviewPanelApi } from 'dockview-core';
import { t, type I18nKey } from '../i18n';
import { terminalStatus, type TerminalStatus } from '../terminal';
import { createSolidPart } from './solidPart';

const STATUS_LABEL: Record<TerminalStatus, I18nKey> = {
  stopped: 'terminal.status.stopped',
  starting: 'terminal.status.starting',
  running: 'terminal.status.running',
  exited: 'terminal.status.exited',
};

function TerminalTab(props: { api: DockviewPanelApi; title: string }) {
  const [title, setTitle] = createSignal(props.title);
  const subscription = props.api.onDidTitleChange((event) => setTitle(event.title));
  onCleanup(() => subscription.dispose());
  const status = () => terminalStatus(props.api.id);

  return (
    <>
      <div class="dv-default-tab-content af-terminal-tab">
        <span
          class={`af-status-dot ${status()}`}
          role="img"
          aria-label={t(STATUS_LABEL[status()])}
          title={t(STATUS_LABEL[status()])}
        />
        {title()}
      </div>
      <div
        class="dv-default-tab-action"
        role="button"
        tabindex="-1"
        aria-label={t('panel.close')}
        onPointerDown={(event) => event.preventDefault()}
        onClick={(event) => {
          if (event.defaultPrevented) return;
          event.preventDefault();
          props.api.close();
        }}
      >
        {createCloseButton()}
      </div>
    </>
  );
}

export function createTerminalTab(): ITabRenderer {
  const part = createSolidPart('dv-default-tab');
  return {
    element: part.element,
    init: (params) => part.mount(() => <TerminalTab api={params.api} title={params.title} />),
    dispose: part.dispose,
  };
}
