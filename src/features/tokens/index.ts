// Token usage and cost (phase 6), core feature: formats, the live usage store (one
// subscription), per-panel badges and budget alerts. Imports shared, i18n, settings and
// providers only. The dashboard (UsageView, lazy UsagePanel) is composed by app/ and layout/.
import type { PtyKind } from '../../shared/types';

export {
  APPROX,
  badgeText,
  costText,
  estimateText,
  formatCost,
  formatShare,
  formatTokens,
  NO_PRICE,
  totalTokens,
} from './format';
export { createPanelUsage } from './panelUsage';
export {
  budgetCovers,
  isCliScope,
  onBudgetAlert,
  onPanelUsage,
  onRepriced,
  reloadUsage,
  scopeLabel,
  startUsage,
  usageSummary,
} from './usageStore';

/** The usage key of an agent CLI (budgets and the ledger use it as the provider). */
export function cliKeyOf(kind: PtyKind): string | null {
  switch (kind) {
    case 'claude':
      return 'claude-code';
    case 'codex':
      return 'codex';
    case 'gemini':
      return 'gemini-cli';
    default:
      return null;
  }
}
