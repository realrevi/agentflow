import { describe, expect, it } from 'vitest';
import type { UsageSettings } from '../../../shared/types';
import { DEFAULT_SETTINGS } from '../defaults';
import {
  moneyText,
  parseMoney,
  percentFromShare,
  removeOverride,
  setFree,
  setProviderBudget,
  shareFromPercent,
  upsertOverride,
} from './usageForm';

describe('usage & budgets settings', () => {
  it('round trips every field of the section', () => {
    const start: UsageSettings = structuredClone(DEFAULT_SETTINGS.usage);
    let next: UsageSettings = { ...start, freeProviders: setFree(start.freeProviders, 'p1', true) };
    next = {
      ...next,
      budgets: setProviderBudget(
        { ...next.budgets, daily: parseMoney('5'), monthly: parseMoney('50,5'), hardStop: true },
        'codex',
        { monthly: parseMoney('30') },
      ),
      prices: upsertOverride(next.prices, {
        model: ' my-model ',
        input: 1,
        output: 2,
        cacheRead: 0.1,
        cacheWrite: 1.25,
      }),
    };
    expect(next).toEqual({
      ...start,
      freeProviders: ['p1'],
      prices: [{ model: 'my-model', input: 1, output: 2, cacheRead: 0.1, cacheWrite: 1.25 }],
      budgets: {
        daily: 5,
        monthly: 50.5,
        hardStop: true,
        providers: [{ provider: 'codex', daily: null, monthly: 30 }],
      },
    });
    // The fields show what is stored, and storing them again changes nothing.
    expect(parseMoney(moneyText(next.budgets.daily))).toBe(5);
    expect(parseMoney(moneyText(null))).toBeNull();

    // And everything can be undone back to the start.
    const back: UsageSettings = {
      ...next,
      freeProviders: setFree(next.freeProviders, 'p1', false),
      prices: removeOverride(next.prices, 'my-model'),
      budgets: {
        ...setProviderBudget(next.budgets, 'codex', { monthly: null }),
        daily: null,
        monthly: null,
        hardStop: false,
      },
    };
    expect(back).toEqual(start);
  });

  it('rejects bad amounts and clamps the threshold', () => {
    expect(parseMoney('')).toBeNull();
    expect(parseMoney('-1')).toBeNull();
    expect(parseMoney('abc')).toBeNull();
    expect(parseMoney('20000', 10_000)).toBeNull();
    expect(percentFromShare(0.7)).toBe(70);
    expect(shareFromPercent(70)).toBe(0.7);
    expect(shareFromPercent(10)).toBe(0.3);
    expect(shareFromPercent(99)).toBe(0.95);
  });

  it('replaces an override in place and ignores empty models', () => {
    const one = upsertOverride([], {
      model: 'a',
      input: 1,
      output: 1,
      cacheRead: 0,
      cacheWrite: 0,
    });
    const two = upsertOverride(one, {
      model: 'b',
      input: 2,
      output: 2,
      cacheRead: 0,
      cacheWrite: 0,
    });
    const edited = upsertOverride(two, { ...two[0]!, input: 9 });
    expect(edited.map((p) => [p.model, p.input])).toEqual([
      ['a', 9],
      ['b', 2],
    ]);
    expect(upsertOverride(two, { ...two[0]!, model: '  ' })).toEqual(two);
  });
});
