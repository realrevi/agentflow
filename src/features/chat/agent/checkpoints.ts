// A chat's per-turn checkpoints: which turns changed files (for "N files changed" and the
// header menu) and "Undo this turn": confirm, restore, refresh (the turn shows as undone).
import { createSignal } from 'solid-js';
import { askChoice, type DialogRequest } from '../../../shared/dialog';
import { ipc } from '../../../shared/ipc';
import { showToast } from '../../../shared/toast';
import type { CheckpointSummary, RestoreReport } from '../../../shared/types';
import { reportError, t } from '../../i18n';

export interface CheckpointDeps {
  list(workspace: string | null, conversationId: string): Promise<CheckpointSummary[]>;
  restore(workspace: string | null, conversationId: string, turn: string): Promise<RestoreReport>;
  confirm(request: DialogRequest): Promise<string>;
  /** Files the undo changed (the tree marks them). */
  changed(paths: readonly string[]): void;
}

const defaultDeps = (changed: (paths: readonly string[]) => void): CheckpointDeps => ({
  list: ipc.agentCheckpoints,
  restore: ipc.agentCheckpointRestore,
  confirm: askChoice,
  changed,
});

export function createCheckpoints(
  workspace: string | null,
  conversationId: string,
  deps: CheckpointDeps | ((paths: readonly string[]) => void),
) {
  const use = typeof deps === 'function' ? defaultDeps(deps) : deps;
  const [list, setList] = createSignal<readonly CheckpointSummary[]>([]);
  const [undoing, setUndoing] = createSignal<string | null>(null);

  async function refresh(): Promise<void> {
    if (workspace === null) return;
    try {
      setList(await use.list(workspace, conversationId));
    } catch (err) {
      reportError('error.checkpoints', err);
    }
  }

  const byTurn = (turn: string) => list().find((checkpoint) => checkpoint.turn === turn);

  /** Resolves `true` once the turn was undone. */
  async function undo(turn: string): Promise<boolean> {
    const checkpoint = byTurn(turn);
    if (!checkpoint || checkpoint.revertedAt !== null || undoing()) return false;
    const choice = await use.confirm({
      title: t('agent.undo.confirm'),
      message: t('agent.undo.body', { files: checkpoint.files.join('\n') }),
      choices: [
        { id: 'cancel', label: t('dialog.cancel'), kind: 'secondary' },
        { id: 'undo', label: t('agent.undo.run'), kind: 'danger' },
      ],
      cancelId: 'cancel',
    });
    if (choice !== 'undo') return false;
    setUndoing(turn);
    try {
      const report = await use.restore(workspace, conversationId, turn);
      use.changed([...report.restored, ...report.removed]);
      showToast({
        kind: 'success',
        title: t('agent.undo.done', {
          restored: report.restored.length,
          removed: report.removed.length,
        }),
      });
      await refresh();
      return true;
    } catch (err) {
      reportError('error.undo', err);
      return false;
    } finally {
      setUndoing(null);
    }
  }

  return { list, byTurn, refresh, undo, undoing };
}

export type Checkpoints = ReturnType<typeof createCheckpoints>;
