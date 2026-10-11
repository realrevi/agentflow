// Small labels for messages: errors by kind, stop reasons, usage.
import type { ChatError, ChatErrorKind, ChatMessage } from '../../shared/types';
import { t, type I18nKey } from '../i18n';

const ERROR_LABEL: Record<ChatErrorKind, I18nKey> = {
  auth: 'chat.error.auth',
  rateLimit: 'chat.error.rateLimit',
  invalidModel: 'chat.error.invalidModel',
  badRequest: 'chat.error.badRequest',
  overloaded: 'chat.error.overloaded',
  server: 'chat.error.server',
  network: 'chat.error.network',
  timeout: 'chat.error.timeout',
  config: 'chat.error.config',
};

/** "Rate limited. Try again in 7 s." plus the provider's own (redacted) words. */
export function errorText(error: ChatError): string {
  const label =
    error.kind === 'rateLimit' && error.retryAfter !== null
      ? t('chat.error.rateLimitWait', { seconds: error.retryAfter })
      : t(ERROR_LABEL[error.kind]);
  return error.message ? `${label} ${error.message}` : label;
}

/** A note under a reply that did not simply finish. */
export function stopNote(message: ChatMessage): string | null {
  if (message.status === 'cancelled') return t('chat.stopped');
  switch (message.stopReason) {
    case 'max_tokens':
    case 'length':
    case 'model_context_window_exceeded':
      return t('chat.cutOff');
    case 'refusal':
    case 'content_filter':
      return t('chat.declined');
    default:
      return null;
  }
}

const number = new Intl.NumberFormat();

export function usageText(message: ChatMessage): string | null {
  const usage = message.usage;
  if (!usage) return null;
  const input = usage.inputTokens + usage.cacheReadTokens + usage.cacheWriteTokens;
  const base = t('chat.usage', {
    input: number.format(input),
    output: number.format(usage.outputTokens),
  });
  return usage.cacheReadTokens > 0
    ? `${base} · ${t('chat.usage.cached', { count: number.format(usage.cacheReadTokens) })}`
    : base;
}
