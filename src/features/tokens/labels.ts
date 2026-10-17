// Display names and theme colors of usage sources (chart, tables, budget scopes).
import type { UsageSource } from '../../shared/types';
import type { I18nKey } from '../i18n';

export const SOURCE_LABEL: Record<UsageSource, I18nKey> = {
  api: 'usage.source.api',
  'claude-code': 'usage.source.claude-code',
  codex: 'usage.source.codex',
  'gemini-cli': 'usage.source.gemini-cli',
};

/** Theme variables only (themes decide the actual colors). */
export const SOURCE_COLOR: Record<UsageSource, string> = {
  api: 'var(--af-accent)',
  'claude-code': 'var(--af-ansi-yellow)',
  codex: 'var(--af-ansi-cyan)',
  'gemini-cli': 'var(--af-ansi-blue)',
};
