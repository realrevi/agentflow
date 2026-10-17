// Files an API agent wrote or edited in this session: the tree marks them until the app
// restarts. Keyed by workspace root, so switching folders keeps each set apart.
import { createStore } from 'solid-js/store';

const [marks, setMarks] = createStore<Record<string, Record<string, true>>>({});

export function markAgentChanged(root: string, paths: readonly string[]): void {
  if (!marks[root]) setMarks(root, {});
  for (const path of paths) setMarks(root, path, true);
}

/** Reactive. */
export function agentChanged(root: string | null, path: string): boolean {
  return root !== null && marks[root]?.[path] === true;
}
