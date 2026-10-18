// The sidebar's "Workstreams" section: every workstream, expandable, with its mode (switchable),
// what it waits for, and each panel's status (working / waiting for you / done / error /
// paused). Clicking a panel focuses it.
import { createSignal, For, Show } from 'solid-js';
import type { HandoffMode } from '../../shared/types';
import { agentState, attentionOf } from '../agentui';
import { Avatar, spriteFor, type CharacterRef } from '../characters';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { reasonText } from './HandoffCards';
import { memberName, type Workstream } from './model';
import { dispatch, removeWorkstream, workstreams } from './store';
import { memberStatus, type MemberStatus } from './status';

export interface MemberView {
  title: string;
  character: CharacterRef | null;
}

export interface WorkstreamsSectionProps {
  panel: (id: string) => MemberView | undefined;
  onFocus: (id: string) => void;
  onTimeline: (workstream: string) => void;
  onNew: () => void;
}

const MODES: { mode: HandoffMode; key: I18nKey }[] = [
  { mode: 'manual', key: 'workstream.mode.manual' },
  { mode: 'semiAuto', key: 'workstream.mode.semiAuto' },
  { mode: 'auto', key: 'workstream.mode.auto' },
];

const STATUS: Record<MemberStatus, I18nKey> = {
  working: 'workstream.status.working',
  waiting: 'workstream.status.waiting',
  done: 'workstream.status.done',
  error: 'workstream.status.error',
  paused: 'workstream.status.paused',
  idle: 'workstream.status.idle',
};

export function ModeSwitch(props: { ws: Workstream }) {
  return (
    <div class="af-mode-pill" role="radiogroup" aria-label={t('workstream.mode')}>
      <For each={MODES}>
        {(entry) => (
          <button
            type="button"
            role="radio"
            aria-checked={props.ws.mode === entry.mode}
            classList={{ active: props.ws.mode === entry.mode }}
            onClick={() => dispatch(props.ws.id, { type: 'mode', mode: entry.mode })}
          >
            {t(entry.key)}
          </button>
        )}
      </For>
    </div>
  );
}

function Member(
  props: { ws: Workstream; id: string; role: 'manager' | 'worker' } & WorkstreamsSectionProps,
) {
  const view = () => props.panel(props.id);
  const status = () =>
    memberStatus(props.ws, props.id, agentState(props.id), attentionOf(props.id) !== null);
  return (
    <li>
      <button
        type="button"
        class="af-list-item af-ws-member"
        onClick={() => props.onFocus(props.id)}
      >
        <Show when={view()?.character} fallback={<span class="af-ws-chip" aria-hidden="true" />}>
          {(character) => (
            <Avatar
              character={character()}
              state={spriteFor(agentState(props.id))}
              scale={1}
              speed={settings.characters.speed}
            />
          )}
        </Show>
        <span class="af-ws-member-name">
          {memberName(props.ws, props.id)}
          <span class="af-ws-member-role">
            {props.role === 'manager' ? t('workstream.manager') : view()?.title}
          </span>
        </span>
        <span class={`af-ws-status ${status()}`}>{t(STATUS[status()])}</span>
        <Show when={attentionOf(props.id)}>
          <span class="af-attention-dot" role="img" aria-label={t('agent.attention')} />
        </Show>
      </button>
    </li>
  );
}

function Item(props: { ws: Workstream } & WorkstreamsSectionProps) {
  const [open, setOpen] = createSignal(true);
  const running = () => props.ws.mode === 'auto' && props.ws.status === 'running';
  return (
    <li class="af-ws-item" style={{ '--af-ws-color': props.ws.color }}>
      <div class="af-ws-head">
        <button
          type="button"
          class="af-ws-toggle"
          aria-expanded={open()}
          onClick={() => setOpen(!open())}
        >
          <span class="af-ws-chip" aria-hidden="true" />
          {props.ws.name}
        </button>
        <button type="button" class="af-link-btn" onClick={() => props.onTimeline(props.ws.id)}>
          {t('workstream.timeline')}
        </button>
      </div>
      <Show when={open()}>
        <div class="af-ws-body">
          <ModeSwitch ws={props.ws} />
          <Show when={reasonText(props.ws)}>{(text) => <p class="af-ws-reason">{text()}</p>}</Show>
          <ul class="af-list">
            <Member {...props} id={props.ws.manager} role="manager" />
            <For each={props.ws.workers}>{(id) => <Member {...props} id={id} role="worker" />}</For>
          </ul>
          <div class="af-button-row">
            <Show
              when={running()}
              fallback={
                <Show when={props.ws.mode === 'auto' && props.ws.status === 'paused'}>
                  <button
                    type="button"
                    class="af-button small"
                    onClick={() => dispatch(props.ws.id, { type: 'resume' })}
                  >
                    {t('workstream.resume')}
                  </button>
                </Show>
              }
            >
              <button
                type="button"
                class="af-button danger small"
                onClick={() => dispatch(props.ws.id, { type: 'stop' })}
              >
                {t('workstream.stop')}
              </button>
            </Show>
            <button
              type="button"
              class="af-button secondary small"
              onClick={() => removeWorkstream(props.ws.id)}
            >
              {t('workstream.remove')}
            </button>
          </div>
        </div>
      </Show>
    </li>
  );
}

export function WorkstreamsSection(props: WorkstreamsSectionProps) {
  return (
    <section class="af-ws-section">
      <h2 class="af-view-heading">{t('workstream.section')}</h2>
      <Show
        when={workstreams().length > 0}
        fallback={<p class="af-view-empty">{t('workstream.none')}</p>}
      >
        <ul class="af-ws-list">
          <For each={workstreams()}>{(ws) => <Item {...props} ws={ws} />}</For>
        </ul>
      </Show>
      <div class="af-button-column">
        <button type="button" class="af-button secondary" onClick={() => props.onNew()}>
          {t('workstream.new')}
        </button>
      </div>
    </section>
  );
}
