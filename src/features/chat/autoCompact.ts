// Auto-compact fires once per threshold crossing: when the estimate first reaches the
// threshold (with auto-compact on), not again until the context drops below it (a compact
// that worked) and rises again.
import type { Estimate } from '../../shared/types';

export function createAutoCompact() {
  let armed = true;
  return {
    /** Feed every new estimate; returns true when a compact should start now. */
    observe(estimate: Estimate | null): boolean {
      if (!estimate) return false;
      if (estimate.share < estimate.threshold) {
        armed = true;
        return false;
      }
      if (!estimate.shouldCompact || !armed) return false;
      armed = false;
      return true;
    },
  };
}
