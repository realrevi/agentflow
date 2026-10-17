// An agent's permission request, inline in the chat: what it wants to do (with the file
// change as before/after), then Allow / Allow for this conversation / Deny with an optional
// reason the model gets.
import { createSignal, Show } from 'solid-js';
import type { PermissionAnswer, PermissionRequest } from '../../shared/types';
import { t } from '../i18n';
import { callTarget, statusText } from './agentTurn';

export function PermissionRow(props: {
  request: PermissionRequest;
  onAnswer: (answer: PermissionAnswer) => void;
}) {
  const [denying, setDenying] = createSignal(false);
  const [reason, setReason] = createSignal('');
  const call = () => ({
    id: props.request.callId,
    name: props.request.tool,
    input: props.request.input,
  });
  const deny = () => {
    const text = reason().trim();
    props.onAnswer({ decision: 'deny', reason: text || null });
  };

  return (
    <div class="af-chat-permission" classList={{ dangerous: Boolean(props.request.dangerous) }}>
      <p class="af-chat-permission-title">
        {t('agent.permission.title', { action: statusText(call()) })}
      </p>
      <Show when={props.request.category === 'command'}>
        <pre class="af-chat-permission-command">{callTarget(call())}</pre>
      </Show>
      <Show when={props.request.dangerous}>
        {(why) => (
          <p class="af-chat-permission-danger">{t('agent.permission.dangerous', { why: why() })}</p>
        )}
      </Show>
      <Show when={props.request.preview}>
        {(preview) => (
          <details class="af-chat-permission-preview">
            <summary>{t('agent.permission.preview', { path: preview().path })}</summary>
            <Show
              when={preview().before !== null}
              fallback={<p class="af-chat-permission-new">{t('agent.permission.newFile')}</p>}
            >
              <pre class="before">{preview().before}</pre>
            </Show>
            <pre class="after">{preview().after}</pre>
          </details>
        )}
      </Show>
      <Show
        when={denying()}
        fallback={
          <div class="af-chat-permission-actions">
            <button
              type="button"
              class="af-button primary small"
              onClick={() => props.onAnswer({ decision: 'allowOnce' })}
            >
              {t('agent.permission.allow')}
            </button>
            <button
              type="button"
              class="af-button secondary small"
              title={t('agent.permission.allowConversationHint')}
              onClick={() => props.onAnswer({ decision: 'allowConversation' })}
            >
              {t('agent.permission.allowConversation')}
            </button>
            <button type="button" class="af-button danger small" onClick={() => setDenying(true)}>
              {t('agent.permission.deny')}
            </button>
          </div>
        }
      >
        <form
          class="af-chat-permission-actions"
          onSubmit={(event) => {
            event.preventDefault();
            deny();
          }}
        >
          <input
            class="af-input"
            placeholder={t('agent.permission.reason')}
            value={reason()}
            onInput={(event) => setReason(event.currentTarget.value)}
            ref={(input) => queueMicrotask(() => input.focus())}
          />
          <button type="submit" class="af-button danger small">
            {t('agent.permission.deny')}
          </button>
        </form>
      </Show>
    </div>
  );
}
