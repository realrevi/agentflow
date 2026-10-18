// Notices above and under the chat's prompt bar: the hard budget stop with "Allow once",
// the context-limit strip with "Compact", and the live pre-send estimate.
import { Show } from 'solid-js';
import type { Estimate } from '../../shared/types';
import { t } from '../i18n';
import { estimateText, formatShare, formatTokens } from '../tokens';
import type { BlockedSend } from './budgetStop';
import { Button } from '../../shared/ui/Button';

export function BudgetNotice(props: {
  blocked: BlockedSend;
  allowing: boolean;
  onAllow: () => void;
  onCancel: () => void;
}) {
  return (
    <div class="af-chat-notice danger" role="alert">
      <div class="af-chat-notice-text">
        <strong>{t('chat.budget.title')}</strong>
        <span>{props.blocked.message}</span>
        <span class="af-chat-notice-hint">{t('chat.budget.hint')}</span>
      </div>
      <div class="af-chat-notice-actions">
        <Button size="sm" disabled={props.allowing} onClick={() => props.onAllow()}>
          {t('chat.budget.allowOnce')}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={props.allowing}
          onClick={() => props.onCancel()}
        >
          {t('chat.budget.cancel')}
        </Button>
      </div>
    </div>
  );
}

/** Shown once the request would fill the context window up to the compact threshold. */
export function ContextStrip(props: {
  estimate: Estimate;
  compacting: boolean;
  onCompact: () => void;
}) {
  return (
    <div class="af-chat-notice warning" role="status">
      <span class="af-chat-notice-text">
        {t('chat.context.warning', {
          share: formatShare(props.estimate.share),
          window: formatTokens(props.estimate.contextWindow),
        })}
      </span>
      <Button size="sm" disabled={props.compacting} onClick={() => props.onCompact()}>
        {props.compacting ? t('chat.compact.running') : t('chat.compact.action')}
      </Button>
    </div>
  );
}

export function EstimateLine(props: { estimate: Estimate | null }) {
  return (
    <Show when={props.estimate}>
      {(estimate) => (
        <div
          class="af-chat-estimate"
          title={t(estimate().approximate ? 'chat.estimate.approxHint' : 'chat.estimate.hint')}
        >
          <span>{t('chat.estimate.label', { value: estimateText(estimate()) })}</span>
          <span class="af-chat-estimate-share">
            {t('chat.estimate.share', {
              share: formatShare(estimate().share),
              window: formatTokens(estimate().contextWindow),
            })}
          </span>
        </div>
      )}
    </Show>
  );
}
