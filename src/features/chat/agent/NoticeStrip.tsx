// Agent notices: tools not supported (info strip) and the step limit (with "Continue").
import { For, Show } from 'solid-js';
import { t } from '../../i18n';
import type { AgentNotice } from './liveTurn';

export function NoticeStrip(props: {
  notices: readonly AgentNotice[];
  canContinue: boolean;
  onContinue: () => void;
}) {
  return (
    <For each={props.notices}>
      {(notice) => (
        <div class={`af-notice-strip ${notice.kind}`} role="status" title={notice.detail}>
          <span>
            {t(
              notice.kind === 'maxIterations'
                ? 'agent.notice.maxIterations'
                : 'agent.notice.noTools',
            )}
          </span>
          <Show when={notice.kind === 'maxIterations' && props.canContinue}>
            <button
              type="button"
              class="af-button primary small"
              onClick={() => props.onContinue()}
            >
              {t('agent.notice.continue')}
            </button>
          </Show>
        </div>
      )}
    </For>
  );
}
