// Compact number and money formats for usage: "12.4k" tokens, "$0.08", "—" when nothing
// has a price, "≈" for estimates that are not counted with the provider's own tokenizer.
import type { Estimate, TokenUsage, Totals } from '../../shared/types';

export const NO_PRICE = '—';
export const APPROX = '≈';

/** All four token counts of a usage record or total. */
export function totalTokens(usage: TokenUsage): number {
  return usage.inputTokens + usage.outputTokens + usage.cacheReadTokens + usage.cacheWriteTokens;
}

function trimmed(value: number, digits: number): string {
  return value.toFixed(digits).replace(/\.0+$/, '');
}

/** 999 · 1.2k · 12.4k · 124k · 1.2M · 3.4B */
export function formatTokens(count: number): string {
  const n = Math.max(0, Math.round(count));
  if (n < 1000) return String(n);
  const [value, unit] = n < 1e6 ? [n / 1e3, 'k'] : n < 1e9 ? [n / 1e6, 'M'] : [n / 1e9, 'B'];
  // 999.95k rounds to "1000k": show the next unit instead.
  if (value >= 999.95 && unit !== 'B') return formatTokens(n < 1e6 ? 1e6 : 1e9);
  return `${trimmed(value, value < 100 ? 1 : 0)}${unit}`;
}

/** $0.00 · <$0.01 · $0.08 · $12.40 · $1.2k */
export function formatCost(usd: number): string {
  const value = Math.max(0, usd);
  if (value === 0) return '$0.00';
  if (value < 0.01) return '<$0.01';
  if (value < 1000) return `$${value.toFixed(2)}`;
  return `$${trimmed(value / 1000, 1)}k`;
}

/** The cost of totals: "—" when none of its tokens has a price. */
export function costText(totals: Pick<Totals, 'cost' | 'unpricedTokens'>): string {
  return totals.cost === 0 && totals.unpricedTokens > 0 ? NO_PRICE : formatCost(totals.cost);
}

/** Some tokens have no price: the cost shown is only the priced part. */
export function hasUnpriced(totals: Pick<Totals, 'cost' | 'unpricedTokens'>): boolean {
  return totals.unpricedTokens > 0;
}

/** The corner badge: "12.4k · $0.08". */
export function badgeText(totals: Totals): string {
  return `${formatTokens(totalTokens(totals))} · ${costText(totals)}`;
}

/** The pre-send estimate: "≈12.4k · $0.04" ("—" without a price). */
export function estimateText(estimate: Estimate): string {
  const tokens = `${estimate.approximate ? APPROX : ''}${formatTokens(estimate.tokens)}`;
  const cost = estimate.cost === null ? NO_PRICE : formatCost(estimate.cost);
  return `${tokens} · ${cost}`;
}

/** 0.734 -> "73%" */
export function formatShare(share: number): string {
  return `${Math.round(Math.max(0, share) * 100)}%`;
}
