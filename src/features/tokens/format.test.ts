import { describe, expect, it } from 'vitest';
import type { Estimate, Totals } from '../../shared/types';
import { badgeText, costText, estimateText, formatCost, formatShare, formatTokens } from './format';

const totals = (patch: Partial<Totals>): Totals => ({
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  requests: 1,
  cost: 0,
  unpricedTokens: 0,
  cacheSavings: 0,
  cliCost: 0,
  ...patch,
});

const estimate = (patch: Partial<Estimate>): Estimate => ({
  tokens: 12_400,
  approximate: false,
  cost: 0.04,
  contextWindow: 200_000,
  windowSource: 'endpoint',
  share: 0.062,
  threshold: 0.7,
  shouldCompact: false,
  ...patch,
});

describe('usage formats', () => {
  it('shortens token counts', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(999)).toBe('999');
    expect(formatTokens(1000)).toBe('1k');
    expect(formatTokens(1234)).toBe('1.2k');
    expect(formatTokens(12_400)).toBe('12.4k');
    expect(formatTokens(124_000)).toBe('124k');
    expect(formatTokens(999_960), 'never "1000k"').toBe('1M');
    expect(formatTokens(1_250_000)).toBe('1.3M');
    expect(formatTokens(3_400_000_000)).toBe('3.4B');
  });

  it('formats dollars', () => {
    expect(formatCost(0)).toBe('$0.00');
    expect(formatCost(0.004)).toBe('<$0.01');
    expect(formatCost(0.08)).toBe('$0.08');
    expect(formatCost(12.4)).toBe('$12.40');
    expect(formatCost(1234)).toBe('$1.2k');
  });

  it('shows "—" only when nothing has a price', () => {
    expect(costText(totals({ cost: 0, unpricedTokens: 500 }))).toBe('—');
    expect(costText(totals({ cost: 0.08, unpricedTokens: 500 }))).toBe('$0.08');
    expect(costText(totals({ cost: 0 }))).toBe('$0.00');
  });

  it('builds the badge and the estimate', () => {
    expect(badgeText(totals({ inputTokens: 12_000, outputTokens: 400, cost: 0.08 }))).toBe(
      '12.4k · $0.08',
    );
    expect(badgeText(totals({ outputTokens: 300, unpricedTokens: 300 }))).toBe('300 · —');
    expect(estimateText(estimate({}))).toBe('12.4k · $0.04');
    expect(estimateText(estimate({ approximate: true, cost: null }))).toBe('≈12.4k · —');
    expect(formatShare(0.734)).toBe('73%');
  });
});
