// A pending permission request: what the agent wants (tool, path or command, category,
// a red badge with the reason for dangerous commands), the change as a diff, a warning
// when the file has unsaved edits, then Allow once / Allow for this conversation (never
// for dangerous commands) / Deny with an optional reason. Nothing is focused or answered
// on its own: only a click or a keypress inside this card answers (see liveTurn.answer).
import { createSignal, Show, Suspense, type Component } from 'solid-js';
import type { PermissionAnswer, PermissionRequest } from '../../../shared/types';
import { PixelIcon } from '../../../shared/ui/PixelIcon';
import { t, type I18nKey } from '../../i18n';
import { toolIcon } from './toolIcons';
import { activeText, durationText, toolTarget } from './toolText';

export interface DiffViewProps {
  path: string;
  before: string | null;
  after: string;
}

export interface PermissionCardProps {
  request: PermissionRequest;
  /** The workspace folder (where commands run). */
  workspace: string | null;
  /** `agent.commandTimeoutSecs`, unless the call sets its own. */
  timeoutSecs: number;
  isDirty: (path: string) => boolean;
  Diff: Component<DiffViewProps>;
  onAnswer: (requestId: number, answer: PermissionAnswer, event: Event) => void;
}

const CATEGORY: Record<PermissionRequest['category'], I18nKey> = {
  read: 'agent.category.read',
  write: 'agent.category.write',
  command: 'agent.category.command',
};

export function PermissionCard(props: PermissionCardProps) {
  const [denying, setDenying] = createSignal(false);
  const [reason, setReason] = createSignal('');
  const call = () => ({
    id: props.request.callId,
    name: props.request.tool,
    input: props.request.input,
  });
  const answer = (value: PermissionAnswer, event: Event) =>
    props.onAnswer(props.request.id, value, event);
  const timeout = () => {
    const own = props.request.input['timeout'];
    return typeof own === 'number' && own > 0 ? own : props.timeoutSecs;
  };

  return (
    <section
      class="af-permission"
      classList={{ dangerous: props.request.dangerous !== null }}
      aria-label={t('agent.attention')}
    >
      <header class="af-permission-head">
        <PixelIcon art={toolIcon(props.request.tool)} class="af-tool-icon" />
        <span class="af-permission-action">{activeText(call())}</span>
        <span class="af-badge">{t(CATEGORY[props.request.category])}</span>
        <Show when={props.request.dangerous}>
          {(why) => (
            <span class="af-badge danger">{t('agent.permission.dangerous', { why: why() })}</span>
          )}
        </Show>
      </header>

      <Show when={props.request.preview}>
        {(preview) => (
          <>
            <Show when={props.isDirty(preview().path)}>
              <p class="af-permission-warning">
                {t('agent.permission.dirty', { path: preview().path })}
              </p>
            </Show>
            <Show when={preview().before === null}>
              <span class="af-badge added">{t('agent.permission.newFile')}</span>
            </Show>
            <div class="af-permission-diff">
              <Suspense fallback={<p class="af-tool-empty">{t('agent.permission.loading')}</p>}>
                <props.Diff
                  path={preview().path}
                  before={preview().before}
                  after={preview().after}
                />
              </Suspense>
            </div>
          </>
        )}
      </Show>

      <Show when={props.request.category === 'command'}>
        <pre class="af-tool-pre af-permission-command">{toolTarget(call())}</pre>
        <p class="af-permission-meta">
          <span>{t('agent.permission.cwd', { path: props.workspace ?? '—' })}</span>
          <span>{t('agent.permission.timeout', { duration: durationText(timeout() * 1000) })}</span>
        </p>
      </Show>

      <Show
        when={denying()}
        fallback={
          <div class="af-permission-actions">
            <button
              type="button"
              class="af-button primary small"
              onClick={(event) => answer({ decision: 'allowOnce' }, event)}
            >
              {t('agent.permission.allow')}
            </button>
            <Show when={props.request.dangerous === null}>
              <button
                type="button"
                class="af-button secondary small"
                title={t('agent.permission.allowConversationHint')}
                onClick={(event) => answer({ decision: 'allowConversation' }, event)}
              >
                {t('agent.permission.allowConversation')}
              </button>
            </Show>
            <button type="button" class="af-button danger small" onClick={() => setDenying(true)}>
              {t('agent.permission.deny')}
            </button>
          </div>
        }
      >
        <form
          class="af-permission-actions"
          onSubmit={(event) => {
            event.preventDefault();
            const text = reason().trim();
            answer({ decision: 'deny', reason: text || null }, event);
          }}
        >
          <input
            class="af-input"
            aria-label={t('agent.permission.reason')}
            placeholder={t('agent.permission.reason')}
            value={reason()}
            onInput={(event) => setReason(event.currentTarget.value)}
          />
          <button type="submit" class="af-button danger small">
            {t('agent.permission.deny')}
          </button>
        </form>
      </Show>
    </section>
  );
}
