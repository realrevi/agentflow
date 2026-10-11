export interface Debounced {
  (): void;
  /** Runs a pending call immediately (e.g. before the window closes). */
  flush(): void;
}

/** Trailing-edge debounce for event-driven saves (no polling anywhere). */
export function debounce(fn: () => void, waitMs: number): Debounced {
  let timer: number | undefined;
  const run = () => {
    timer = undefined;
    fn();
  };
  const debounced = (() => {
    window.clearTimeout(timer);
    timer = window.setTimeout(run, waitMs);
  }) as Debounced;
  debounced.flush = () => {
    if (timer === undefined) return;
    window.clearTimeout(timer);
    run();
  };
  return debounced;
}
