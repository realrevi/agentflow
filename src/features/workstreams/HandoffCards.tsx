// The floating cards of workstreams that wait for the user: a pending handoff as an
// editable preview (semi-auto: open at once; manual: the send button first, the text one
// click away) and why a workstream paused, with Resume. Handoff text is shown as plain text
// in a textarea, never rendered or run.
import { createSignal, For, Show } from 'solid-js';
import { t, type I18nKey } from '../i18n';
import type { Handoff, PauseReason, Workstream } from './model';
import { memberName } from './model';
import { remindManager } from './runtime';
import { dispatch, setPendingText, workstreams } from './store';
import { Button } from '../../shared/ui/Button';
import { Textarea } from '../../shared/ui/Field';

const REASONS: Partial<Record<PauseReason, I18nKey>> = {
  ask: 'workstream.reason.ask',
  done: 'workstream.reason.done',
  permission: 'workstream.reason.permission',
  error: 'workstream.reason.error',
  malformed: 'workstream.reason.malformed',
  noBlock: 'workstream.reason.noBlock',
  unknownWorker: 'workstream.reason.unknownWorker',
  maxRounds: 'workstream.reason.maxRounds',
  budget: 'workstream.reason.budget',
  userTyped: 'workstream.reason.userTyped',
  stopped: 'workstream.reason.stopped',
  notRunning: 'workstream.reason.notRunning',
  interrupted: 'workstream.reason.interrupted',
  restored: 'workstream.reason.restored',
};

/** "Paused: …" for a workstream, or null when it runs or simply waits for a preview. */
export function reasonText(ws: Workstream): string | null {
  const key = ws.reason ? REASONS[ws.reason] : undefined;
  return key ? t(key, { detail: ws.detail ?? '', name: ws.name }) : null;
}

function sendLabel(handoff: Handoff): string {
  if (handoff.kind === 'report') return t('workstream.sendReport');
  if (handoff.kind === 'setup') return t('workstream.sendSetup');
  return t('workstream.sendPrompt');
}

function Card(props: { ws: Workstream; onFocus: (panel: string) => void }) {
  const [editing, setEditing] = createSignal(props.ws.mode === 'semiAuto');
  const pending = () => props.ws.pending;
  const reason = () => reasonText(props.ws);
  const canResume = () =>
    props.ws.status === 'paused' && props.ws.reason !== 'done' && props.ws.mode === 'auto';
  return (
    <section
      class="af-handoff-card"
      style={{ '--af-ws-color': props.ws.color }}
      aria-label={t('workstream.card', { name: props.ws.name })}
    >
      <header class="af-handoff-head">
        <span class="af-ws-chip" aria-hidden="true" />
        <strong>{props.ws.name}</strong>
        <Show when={pending()}>
          {(handoff) => (
            <Button variant="ghost" size="sm" onClick={() => props.onFocus(handoff().from)}>
              {t('workstream.route', {
                from: memberName(props.ws, handoff().from),
                to: memberName(props.ws, handoff().to),
              })}
            </Button>
          )}
        </Show>
      </header>
      <Show when={reason()}>
        {(text) => (
          <p class="af-handoff-reason">
            {text()}
            <Show when={props.ws.warning === 'multiple'}>
              <span class="af-handoff-warning"> {t('workstream.warning.multiple')}</span>
            </Show>
          </p>
        )}
      </Show>
      <Show when={pending()}>
        {(handoff) => (
          <>
            <Show when={editing()}>
              <Textarea
                class="af-handoff-text"
                rows={6}
                spellcheck={false}
                aria-label={t('workstream.preview')}
                value={handoff().text}
                onInput={(event) => setPendingText(props.ws.id, event.currentTarget.value)}
              />
              <Show when={handoff().truncated}>
                <p class="af-handoff-note">{t('workstream.truncated')}</p>
              </Show>
            </Show>
            <div class="af-button-row">
              <Button
                size="sm"
                onClick={() => dispatch(props.ws.id, { type: 'send', text: handoff().text })}
              >
                {sendLabel(handoff())}
              </Button>
              <Show when={!editing()}>
                <Button variant="secondary" size="sm" onClick={() => setEditing(true)}>
                  {t('workstream.edit')}
                </Button>
              </Show>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => dispatch(props.ws.id, { type: 'dismiss' })}
              >
                {t('workstream.discard')}
              </Button>
            </div>
          </>
        )}
      </Show>
      <Show when={!pending() && (props.ws.reason === 'noBlock' || props.ws.reason === 'malformed')}>
        <div class="af-button-row">
          <Button size="sm" onClick={() => void remindManager(props.ws.id)}>
            {t('workstream.remind', { name: memberName(props.ws, props.ws.manager) })}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => props.onFocus(props.ws.manager)}>
            {t('workstream.openManager')}
          </Button>
        </div>
      </Show>
      <Show when={!pending() && props.ws.reason === 'ask'}>
        <div class="af-button-row">
          <Button size="sm" onClick={() => props.onFocus(props.ws.manager)}>
            {t('workstream.answer', { name: memberName(props.ws, props.ws.manager) })}
          </Button>
        </div>
      </Show>
      <Show when={!pending() && canResume()}>
        <div class="af-button-row">
          <Button size="sm" onClick={() => dispatch(props.ws.id, { type: 'resume' })}>
            {t('workstream.resume')}
          </Button>
        </div>
      </Show>
    </section>
  );
}

/** Workstreams that wait for the user: a pending handoff, or a pause with a reason. */
export function needsCard(ws: Workstream): boolean {
  if (ws.pending || ws.reason === 'ask') return true;
  return ws.status === 'paused' && ws.reason !== null && ws.reason !== 'restored';
}

export function HandoffCards(props: { onFocus: (panel: string) => void }) {
  return (
    <div class="af-handoff-stack">
      <For each={workstreams().filter(needsCard)}>
        {(ws) => <Card ws={ws} onFocus={props.onFocus} />}
      </For>
    </div>
  );
}
