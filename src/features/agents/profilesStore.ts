// Installed AgentFlow agents of the current workspace (`.agentflow/agents/*.md`), for the
// "+" menu and the empty agent area. Reloaded when the workspace changes, after installs
// and when files in that folder change.
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { AgentProfile } from '../../shared/types';
import { currentWorkspace, onFsChanges } from '../workspace';

const PROFILES_DIR = '.agentflow/agents/';

const [profiles, setProfiles] = createSignal<readonly AgentProfile[]>([]);
export { profiles as agentProfiles };

let loadedFor: string | null = null;

export async function refreshProfiles(): Promise<void> {
  const workspace = currentWorkspace();
  loadedFor = workspace;
  try {
    const list = workspace ? await ipc.agentsProfiles(workspace) : [];
    if (loadedFor === workspace) setProfiles(list);
  } catch {
    if (loadedFor === workspace) setProfiles([]);
  }
}

/** Follows the workspace (and, when the watcher reports it, the profile folder).
 * Returns a cleanup function. */
export function watchProfiles(): () => void {
  const dispose = createRoot((done) => {
    createEffect(on(currentWorkspace, () => void refreshProfiles()));
    return done;
  });
  const stop = onFsChanges((changes) => {
    if (changes.rescan || changes.paths.some((path) => path.startsWith(PROFILES_DIR))) {
      void refreshProfiles();
    }
  });
  return () => {
    stop();
    dispose();
  };
}
