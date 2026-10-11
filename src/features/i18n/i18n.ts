import { createSignal } from 'solid-js';
import type { Locale } from '../../shared/types';
import {
  FALLBACK_LOCALE,
  LOADERS,
  fallbackDictionary,
  type Dictionary,
  type I18nKey,
} from './locales';

export type TranslateParams = Record<string, string | number>;

const [locale, setLocaleSignal] = createSignal<Locale>(FALLBACK_LOCALE);
const [dictionary, setDictionary] = createSignal<Dictionary>(fallbackDictionary);

export { locale };

/** Replaces `{name}` placeholders; unknown placeholders are left as-is. */
export function interpolate(template: string, params: TranslateParams): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

/** Reactive: components re-render when the language changes. */
export function t(key: I18nKey, params?: TranslateParams): string {
  const template = dictionary()[key] || fallbackDictionary[key];
  return params ? interpolate(template, params) : template;
}

/** Loads (if needed) and activates a language. Throws if the chunk cannot be loaded. */
export async function setLocale(next: Locale): Promise<void> {
  const dict = await LOADERS[next]();
  setDictionary(() => dict);
  setLocaleSignal(next);
  document.documentElement.lang = next;
}
