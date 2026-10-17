import { describe, expect, it } from 'vitest';
import type { SourceTotals, Totals, UsageDay, UsageSource } from '../../shared/types';
import { defaultMetric, shapeChart, shiftDate } from './chart';

const zero: Totals = {
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  requests: 0,
  cost: 0,
  unpricedTokens: 0,
  cacheSavings: 0,
  cliCost: 0,
};

const slice = (source: UsageSource, cost: number, tokens: number): SourceTotals => ({
  ...zero,
  source,
  requests: 1,
  cost,
  inputTokens: tokens,
});

const day = (date: string, sources: SourceTotals[]): UsageDay => ({
  ...zero,
  date,
  sources,
});

describe('daily chart data', () => {
  it('fills 30 days ending today, missing days as zero bars', () => {
    const chart = shapeChart([day('2026-10-15', [slice('api', 1, 10)])], 'cost', '2026-10-17');
    expect(chart.bars).toHaveLength(30);
    expect(chart.bars[0]?.date).toBe('2026-09-18');
    expect(chart.bars[29]?.date).toBe('2026-10-17');
    expect(chart.bars.filter((bar) => bar.total > 0).map((bar) => bar.date)).toEqual([
      '2026-10-15',
    ]);
    expect(chart.bars[29]?.segments).toEqual([]);
  });

  it('stacks sources in a fixed order, bottom to top', () => {
    const chart = shapeChart(
      [
        day('2026-10-17', [
          slice('codex', 0.5, 500),
          slice('api', 1, 100),
          slice('claude-code', 2, 2000),
        ]),
      ],
      'cost',
      '2026-10-17',
    );
    const today = chart.bars[29];
    expect(today?.segments.map((s) => [s.source, s.y0, s.y1])).toEqual([
      ['api', 0, 1],
      ['claude-code', 1, 3],
      ['codex', 3, 3.5],
    ]);
    expect(today?.total).toBe(3.5);
    expect(chart.max).toBe(3.5);
  });

  it('can count tokens instead, and picks tokens when nothing has a price', () => {
    const chart = shapeChart(
      [day('2026-10-17', [slice('gemini-cli', 0, 700)])],
      'tokens',
      '2026-10-17',
    );
    expect(chart.bars[29]?.total).toBe(700);
    expect(defaultMetric({ ...zero, cost: 0, unpricedTokens: 700 })).toBe('tokens');
    expect(defaultMetric({ ...zero, cost: 0.1 })).toBe('cost');
  });

  it('shifts dates across months and leap days', () => {
    expect(shiftDate('2026-03-01', -1)).toBe('2026-02-28');
    expect(shiftDate('2028-03-01', -1)).toBe('2028-02-29');
    expect(shiftDate('2026-12-31', 1)).toBe('2027-01-01');
  });
});
