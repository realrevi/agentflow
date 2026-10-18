// Translated labels for library ids (formats, kinds, flags, unsupported reasons, source
// errors) and the short tool marks on cards. Unknown ids fall back to a generic text.
import type {
  InstallTarget,
  LibraryFlagId,
  LibraryFormat,
  LibrarySourceError,
  TemplateKind,
} from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';

export const KIND_LABELS: Record<TemplateKind, I18nKey> = {
  agent: 'library.kind.agent',
  skill: 'library.kind.skill',
  rule: 'library.kind.rule',
  command: 'library.kind.command',
};

export const FORMAT_LABELS: Record<LibraryFormat, I18nKey> = {
  claudeSubagent: 'library.format.claudeSubagent',
  claudeSkill: 'library.format.claudeSkill',
  cursorRule: 'library.format.cursorRule',
  codexAgent: 'library.format.codexAgent',
  geminiCommand: 'library.format.geminiCommand',
  geminiAgent: 'library.format.geminiAgent',
  agentsMd: 'library.format.agentsMd',
};

export const FLAG_LABELS: Record<LibraryFlagId, I18nKey> = {
  downloadExec: 'library.flag.downloadExec',
  disablePermissions: 'library.flag.disablePermissions',
  ignoreUser: 'library.flag.ignoreUser',
  sendsData: 'library.flag.sendsData',
  encodedBlob: 'library.flag.encodedBlob',
  executable: 'library.flag.executable',
};

const REASONS: Record<string, I18nKey> = {
  noFrontmatter: 'library.unsupported.noFrontmatter',
  frontmatter: 'library.unsupported.frontmatter',
  empty: 'library.unsupported.empty',
  invalid: 'library.unsupported.invalid',
  tooLarge: 'library.unsupported.tooLarge',
  license: 'library.unsupported.license',
  notFetched: 'library.unsupported.notFetched',
  notText: 'library.unsupported.notText',
  unknownFormat: 'library.unsupported.unknownFormat',
  tooMany: 'library.unsupported.tooMany',
};

export function reasonText(reason: string): string {
  return t(REASONS[reason] ?? 'library.unsupported.other');
}

const ERRORS: Record<string, I18nKey> = {
  notFound: 'library.error.notFound',
  network: 'library.error.network',
  invalid: 'library.error.invalid',
  http: 'library.error.http',
  tooLarge: 'library.error.tooLarge',
};

export function resetTime(at: number): string {
  return new Date(at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function errorText(error: LibrarySourceError): string {
  if (error.kind === 'rateLimited') {
    return t('library.error.rateLimited', { time: resetTime(error.resetAt ?? Date.now()) });
  }
  return t(ERRORS[error.kind] ?? 'library.error.http', { detail: error.message });
}

/** Tool marks on cards (product names are not translated; the title has the full name). */
export const TARGET_MARKS: Record<InstallTarget, string> = {
  claudeCode: 'CC',
  codex: 'CX',
  gemini: 'GE',
  cursor: 'CU',
  agentflow: 'AF',
};
