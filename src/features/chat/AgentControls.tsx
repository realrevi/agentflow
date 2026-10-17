// Chat header controls for API agents: the permission mode of this conversation and the
// checkpoints (undo a turn's file changes).
import { createSignal, Show } from 'solid-js';
import { askChoice } from '../../shared/dialog';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { CheckpointSummary, Conversation, PermissionMode } from '../../shared/types';
import { Menu } from '../../shared/ui/Menu';
import { reportError, t } from '../i18n';
import { settings } from '../settings/settingsStore';

const MODES: readonly PermissionMode[] = ['ask', 'autoEdit', 'readOnly'];
const MODE_LABEL = {
  ask: 'agent.mode.ask',
  autoEdit: 'agent.mode.autoEdit',
  readOnly: 'agent.mode.readOnly',
} as const;

export function AgentControls(props: {
  panelId: string;
  workspace: string | null;
  conversation: Conversation;
  onChanged: (conversation: Conversation) => void;
}) {
  const [menu, setMenu] = createSignal<{ x: number; y: number } | null>(null);
  const [checkpoints, setCheckpoints] = createSignal<readonly CheckpointSummary[]>([]);
  const mode = () => props.conversation.agent.permissionMode ?? settings.agent.defaultMode;

  const changeMode = async (next: PermissionMode) => {
    const before = props.conversation;
    props.onChanged({ ...before, agent: { ...before.agent, permissionMode: next } });
    try {
      await ipc.conversationUpdate(props.workspace, before.id, { permissionMode: next });
    } catch (err) {
      props.onChanged(before);
      reportError('error.conversationUpdate', err);
    }
  };

  const openMenu = async (event: MouseEvent) => {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    try {
      setCheckpoints(await ipc.agentCheckpoints(props.workspace, props.conversation.id));
      setMenu({ x: rect.left, y: rect.bottom });
    } catch (err) {
      reportError('error.checkpoints', err);
    }
  };

  const restore = async (turn: string) => {
    const checkpoint = checkpoints().find((c) => c.turn === turn);
    if (!checkpoint) return;
    const choice = await askChoice({
      title: t('agent.checkpoint.confirm'),
      message: checkpoint.files.join('\n'),
      choices: [
        { id: 'cancel', label: t('dialog.cancel'), kind: 'secondary' },
        { id: 'restore', label: t('agent.checkpoint.restore'), kind: 'danger' },
      ],
      cancelId: 'cancel',
    });
    if (choice !== 'restore') return;
    try {
      const report = await ipc.agentCheckpointRestore(props.workspace, props.conversation.id, turn);
      showToast({
        kind: 'success',
        title: t('agent.checkpoint.restored', {
          restored: report.restored.length,
          removed: report.removed.length,
        }),
      });
    } catch (err) {
      reportError('error.checkpointRestore', err);
    }
  };

  return (
    <>
      <label class="af-visually-hidden" for={`af-chat-${props.panelId}-mode`}>
        {t('agent.mode')}
      </label>
      <select
        id={`af-chat-${props.panelId}-mode`}
        class="af-select af-chat-mode"
        title={t('agent.mode')}
        value={mode()}
        onChange={(event) => void changeMode(event.currentTarget.value as PermissionMode)}
      >
        {MODES.map((value) => (
          <option value={value}>{t(MODE_LABEL[value])}</option>
        ))}
      </select>
      <button
        type="button"
        class="af-chat-system-toggle"
        disabled={!props.workspace}
        onClick={(event) => void openMenu(event)}
      >
        {t('agent.checkpoints')}
      </button>
      <Show when={menu()}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            placement="below-start"
            items={
              checkpoints().length > 0
                ? checkpoints().map((c) => ({
                    id: c.turn,
                    label: t('agent.checkpoint.item', {
                      time: new Date(c.at).toLocaleString(),
                      count: c.files.length,
                    }),
                  }))
                : [{ id: '', label: t('agent.checkpoint.none'), disabled: true }]
            }
            onSelect={(id) => void restore(id)}
            onClose={() => setMenu(null)}
          />
        )}
      </Show>
    </>
  );
}
