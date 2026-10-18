// The tab of every panel: dockview's default look, plus a status dot for terminals (and
// their character, which opens the character picker), a dirty dot for editors (it turns
// into the close button on hover), an attention dot and the gold glow while a panel needs
// the user, a guarded close, and (agent panels) a context menu to link it to a workstream.
import { createCloseButton, type DockviewPanelApi, type ITabRenderer } from 'dockview-core';
import { createSignal, onCleanup, Show } from 'solid-js';
import { Menu } from '../../shared/ui/Menu';
import { Popover } from '../../shared/ui/Popover';
import { Avatar, CharacterPicker, spriteFor } from '../characters';
import { editorDirty } from '../editor';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { agentState, attentionOf, glowing } from '../agentui';
import { terminalStatus, type TerminalStatus } from '../terminal';
import { characterOf } from './characters';
import { closePanel, setPanelCharacter } from './layoutStore';
import { linkMenuItems, selectLinkItem } from './linkMenu';
import { parsePanelParams, type PanelParams } from './panels';
import { createSolidPart } from './solidPart';

const STATUS_LABEL: Record<TerminalStatus, I18nKey> = {
  stopped: 'terminal.status.stopped',
  starting: 'terminal.status.starting',
  running: 'terminal.status.running',
  exited: 'terminal.status.exited',
};

function PanelTab(props: { api: DockviewPanelApi; title: string; params: PanelParams | null }) {
  const [title, setTitle] = createSignal(props.title);
  const [params, setParams] = createSignal(props.params);
  const subscriptions = [
    props.api.onDidTitleChange((event) => setTitle(event.title)),
    props.api.onDidParametersChange((next) => setParams(parsePanelParams(next))),
  ];
  onCleanup(() => subscriptions.forEach((subscription) => subscription.dispose()));

  const status = () => terminalStatus(props.api.id);
  const dirty = () => params()?.kind === 'editor' && editorDirty(props.api.id);
  const tooltip = () => {
    const current = params();
    return current?.kind === 'editor' ? current.path : undefined;
  };
  const character = () => characterOf(params());
  const [picking, setPicking] = createSignal(false);
  const [menuAt, setMenuAt] = createSignal<{ x: number; y: number } | null>(null);
  const isAgent = () => {
    const current = params();
    return (
      current?.kind === 'chat' || (current?.kind === 'terminal' && current.config.kind !== 'shell')
    );
  };
  let avatarButton: HTMLButtonElement | undefined;

  return (
    <>
      <div
        class="dv-default-tab-content af-tab"
        classList={{ 'af-glow-tab': glowing(props.api.id) }}
        title={tooltip()}
        onContextMenu={(event) => {
          if (!isAgent()) return;
          event.preventDefault();
          setMenuAt({ x: event.clientX, y: event.clientY });
        }}
      >
        <Show when={character()}>
          {(current) => (
            <button
              ref={avatarButton}
              type="button"
              class="af-tab-avatar"
              title={t('characters.change')}
              aria-label={t('characters.change')}
              aria-expanded={picking()}
              onClick={(event) => {
                event.stopPropagation();
                setPicking((open) => !open);
              }}
            >
              <Avatar
                character={current()}
                state={spriteFor(agentState(props.api.id))}
                scale={1}
                speed={settings.characters.speed}
              />
            </button>
          )}
        </Show>
        <Show when={params()?.kind === 'terminal'}>
          <span
            class={`af-status-dot ${status()}`}
            role="img"
            aria-label={t(STATUS_LABEL[status()])}
            title={t(STATUS_LABEL[status()])}
          />
        </Show>
        <Show when={attentionOf(props.api.id)}>
          {(reason) => (
            <span
              class="af-attention-dot"
              role="img"
              aria-label={t(`attention.${reason()}`)}
              title={t(`attention.${reason()}`)}
            />
          )}
        </Show>
        {title()}
      </div>
      <Show when={menuAt()}>
        {(at) => (
          <Menu
            anchor={at()}
            items={linkMenuItems(props.api.id)}
            placement="below-start"
            onSelect={(id) => void selectLinkItem(props.api.id, id, title())}
            onClose={() => setMenuAt(null)}
          />
        )}
      </Show>
      <Show when={picking() && avatarButton}>
        {(anchor) => (
          <Popover
            anchor={anchor()}
            placement="below-start"
            label={t('characters.picker.title')}
            onClose={() => setPicking(false)}
          >
            <CharacterPicker
              current={character()}
              onPick={(next) => setPanelCharacter(props.api.id, next)}
            />
          </Popover>
        )}
      </Show>
      <div
        class="dv-default-tab-action af-tab-action"
        classList={{ dirty: dirty() }}
        role="button"
        tabindex="-1"
        title={dirty() ? t('editor.unsaved') : undefined}
        aria-label={dirty() ? `${t('editor.unsaved')}, ${t('panel.close')}` : t('panel.close')}
        onPointerDown={(event) => event.preventDefault()}
        onClick={(event) => {
          if (event.defaultPrevented) return;
          event.preventDefault();
          void closePanel(props.api.id);
        }}
      >
        <span class="af-dirty-dot" aria-hidden="true" />
        {createCloseButton()}
      </div>
    </>
  );
}

export function createPanelTab(): ITabRenderer {
  const part = createSolidPart('dv-default-tab');
  return {
    element: part.element,
    init: (params) =>
      part.mount(() => (
        <PanelTab api={params.api} title={params.title} params={parsePanelParams(params.params)} />
      )),
    dispose: part.dispose,
  };
}
