// Supported languages. English is bundled and is the fallback; others load on demand.
// Each loader is type-checked against `Dictionary`, so a locale missing a key fails `pnpm typecheck`.
import en from '../../locales/en.json';
import type { Locale } from '../../shared/types';

export type Dictionary = typeof en;
export type I18nKey = keyof Dictionary;

export const FALLBACK_LOCALE: Locale = 'en';

/** Language names are shown in their own language, so they are not translated. */
export const LOCALES: readonly { code: Locale; name: string }[] = [
  { code: 'en', name: 'English' },
  { code: 'tr', name: 'Türkçe' },
  { code: 'de', name: 'Deutsch' },
  { code: 'zh', name: '中文' },
  { code: 'ru', name: 'Русский' },
];

export const LOADERS: Record<Locale, () => Promise<Dictionary>> = {
  en: async () => en,
  tr: async () => (await import('../../locales/tr.json')).default,
  de: async () => (await import('../../locales/de.json')).default,
  zh: async () => (await import('../../locales/zh.json')).default,
  ru: async () => (await import('../../locales/ru.json')).default,
};

export { en as fallbackDictionary };
