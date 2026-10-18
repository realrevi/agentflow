// The user typed into an agent panel (its prompt bar or the terminal itself). An auto
// workstream stops when that happens in one of its panels; deliveries do not count.
type Listener = (panelId: string) => void;

const listeners = new Set<Listener>();

export function noteUserInput(panelId: string): void {
  for (const listener of [...listeners]) listener(panelId);
}

export function onUserInput(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
