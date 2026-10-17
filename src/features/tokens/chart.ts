// Data for the daily bar chart: the last N local days (days without usage are zero bars),
// each bar stacked by source in a fixed order, values in cost or tokens.
import type { Totals, UsageDay, UsageSource } from '../../shared/types';
import { totalTokens } from './format';

export const SOURCES: readonly UsageSource[] = ['api', 'claude-code', 'codex', 'gemini-cli'];

export type ChartMetric = 'cost' | 'tokens';

export interface BarSegment {
  source: UsageSource;
  value: number;
  /** Stack bottom and top, in metric units. */
  y0: number;
  y1: number;
}

export interface Bar {
  /** `YYYY-MM-DD` */
  date: string;
  total: number;
  /** Bottom to top, only sources with usage. */
  segments: BarSegment[];
}

export interface Chart {
  bars: Bar[];
  /** The tallest bar (never 0, so empty charts still scale). */
  max: number;
}

export function metricValue(totals: Totals, metric: ChartMetric): number {
  return metric === 'cost' ? totals.cost : totalTokens(totals);
}

/** Cost when anything has a price, else tokens (e.g. only unpriced local models). */
export function defaultMetric(month: Totals): ChartMetric {
  return month.cost > 0 ? 'cost' : 'tokens';
}

/** `YYYY-MM-DD` shifted by whole days (calendar math in UTC, so no DST surprises). */
export function shiftDate(date: string, days: number): string {
  const ms = Date.parse(`${date}T00:00:00Z`) + days * 86_400_000;
  return new Date(ms).toISOString().slice(0, 10);
}

/** Today in local time as `YYYY-MM-DD`. */
export function localDate(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** `count` bars ending at `end`; missing days are zero. */
export function shapeChart(
  days: readonly UsageDay[],
  metric: ChartMetric,
  end: string,
  count = 30,
): Chart {
  const byDate = new Map(days.map((day) => [day.date, day]));
  const bars: Bar[] = [];
  for (let offset = count - 1; offset >= 0; offset--) {
    const date = shiftDate(end, -offset);
    const day = byDate.get(date);
    let top = 0;
    const segments: BarSegment[] = [];
    for (const source of SOURCES) {
      const slice = day?.sources.find((s) => s.source === source);
      const value = slice ? metricValue(slice, metric) : 0;
      if (value <= 0) continue;
      segments.push({ source, value, y0: top, y1: top + value });
      top += value;
    }
    bars.push({ date, total: top, segments });
  }
  const max = Math.max(1e-9, ...bars.map((bar) => bar.total));
  return { bars, max: metric === 'tokens' ? Math.max(1, max) : max };
}
