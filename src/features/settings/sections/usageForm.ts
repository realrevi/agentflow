// Pure helpers for the "Usage & budgets" section: money and percent fields, free
// providers, per-provider budgets and price overrides. Kept apart so the round trip
// (settings -> fields -> settings) is unit tested.
import type { Budgets, PriceOverride, ProviderBudget } from '../../../shared/types';

/** The backend's ranges (settings/chat.rs). */
export const THRESHOLD_PERCENT = { min: 30, max: 95 } as const;
export const KEEP_TURNS = { min: 1, max: 50 } as const;
export const MAX_PRICE = 10_000;

/** A budget or price typed by the user: `null` when empty or not a valid amount. */
export function parseMoney(text: string, max = Number.POSITIVE_INFINITY): number | null {
  const trimmed = text.trim().replace(',', '.');
  if (trimmed === '') return null;
  const value = Number(trimmed);
  return Number.isFinite(value) && value >= 0 && value <= max ? value : null;
}

export function moneyText(value: number | null): string {
  return value === null ? '' : String(value);
}

export function percentFromShare(share: number): number {
  return Math.round(share * 100);
}

/** A percent field -> a share, clamped to the backend's range. */
export function shareFromPercent(percent: number): number {
  const clamped = Math.min(THRESHOLD_PERCENT.max, Math.max(THRESHOLD_PERCENT.min, percent));
  return Math.round(clamped) / 100;
}

export function setFree(list: readonly string[], providerId: string, free: boolean): string[] {
  const rest = list.filter((id) => id !== providerId);
  return free ? [...rest, providerId] : rest;
}

/** Sets one provider's daily/monthly limits; a row with neither is removed. */
export function setProviderBudget(
  budgets: Budgets,
  provider: string,
  patch: Partial<Pick<ProviderBudget, 'daily' | 'monthly'>>,
): Budgets {
  const current = budgets.providers.find((entry) => entry.provider === provider) ?? {
    provider,
    daily: null,
    monthly: null,
  };
  const next = { ...current, ...patch };
  const others = budgets.providers.filter((entry) => entry.provider !== provider);
  const keep = next.daily !== null || next.monthly !== null;
  return { ...budgets, providers: keep ? [...others, next] : others };
}

/** Adds or replaces the override for `override.model` (keeping its place in the list). */
export function upsertOverride(
  prices: readonly PriceOverride[],
  override: PriceOverride,
): PriceOverride[] {
  const model = override.model.trim();
  if (!model) return [...prices];
  const entry = { ...override, model };
  const index = prices.findIndex((price) => price.model === model);
  if (index < 0) return [...prices, entry];
  return prices.map((price, i) => (i === index ? entry : price));
}

export function removeOverride(prices: readonly PriceOverride[], model: string): PriceOverride[] {
  return prices.filter((price) => price.model !== model);
}
