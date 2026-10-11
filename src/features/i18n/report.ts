import { toAppError } from '../../shared/errors';
import { showToast } from '../../shared/toast';
import { t } from './i18n';
import type { I18nKey } from './locales';

/** Shows a translated error toast: `context` says what failed, the backend message says why. */
export function reportError(context: I18nKey, err: unknown): void {
  const error = toAppError(err);
  console.error(`[agentflow] ${context}:`, error);
  showToast({ kind: 'error', title: t(context), detail: error.message });
}
