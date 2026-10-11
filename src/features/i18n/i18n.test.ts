import { describe, expect, it } from 'vitest';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import ru from '../../locales/ru.json';
import tr from '../../locales/tr.json';
import zh from '../../locales/zh.json';
import { interpolate } from './i18n';
import { LOCALES } from './locales';

const dictionaries: Record<string, Record<string, string>> = { en, tr, de, zh, ru };
const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe('locale files', () => {
  it('cover every supported locale', () => {
    expect(Object.keys(dictionaries).sort()).toEqual(LOCALES.map((l) => l.code).sort());
  });

  const enKeys = Object.keys(en).sort();

  for (const [code, dict] of Object.entries(dictionaries)) {
    describe(code, () => {
      it('has exactly the same keys as en.json', () => {
        expect(Object.keys(dict).sort()).toEqual(enKeys);
      });

      it('has no empty translations', () => {
        const empty = Object.entries(dict).filter(([, text]) => text.trim() === '');
        expect(empty).toEqual([]);
      });

      it('keeps the same {placeholders} as English', () => {
        for (const key of enKeys) {
          expect(placeholders(dict[key] ?? ''), `${code}: ${key}`).toEqual(
            placeholders(dictionaries.en?.[key] ?? ''),
          );
        }
      });
    });
  }
});

describe('interpolate', () => {
  it('fills known placeholders and keeps unknown ones', () => {
    expect(interpolate('Agent {n} of {total}', { n: 2 })).toBe('Agent 2 of {total}');
  });
});
