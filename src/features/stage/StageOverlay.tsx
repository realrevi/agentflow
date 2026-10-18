// The Stage's DOM layer over the canvas: workstream labels, character names, speech bubbles
// (typewriter, ~40 characters with the full text as tooltip, fading when the agent rests)
// and the hover tooltip (AI/tool, model, status, session tokens · cost).
import { createResource, For, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { activityOf, agentState, attentionOf, portOf } from '../agentui';
import { SpeechBubble } from '../agentui/overlay/SpeechBubble';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { badgeText } from '../tokens/format';
import { memberStatus, type MemberStatus } from '../workstreams/status';
import { workstreamOf } from '../workstreams/store';
import { bubbleView, type Point } from './scene';
import type { ClusterLabel } from './layout';

const sessionStart = Date.now();

export interface OverlayCharacter {
  id: string;
  name: string;
  at: Point;
}

const STATUS: Record<MemberStatus, I18nKey> = {
  working: 'workstream.status.working',
  waiting: 'workstream.status.waiting',
  done: 'workstream.status.done',
  error: 'workstream.status.error',
  paused: 'workstream.status.paused',
  idle: 'workstream.status.idle',
};

function statusOf(panel: string): string {
  const ws = workstreamOf(panel);
  const state = agentState(panel);
  const needs = attentionOf(panel) !== null;
  if (ws) return t(STATUS[memberStatus(ws, panel, state, needs)]);
  if (state === 'think' || state === 'work') return t(STATUS.working);
  if (needs) return t(STATUS.waiting);
  return t(state === 'error' ? STATUS.error : STATUS.idle);
}

function Tooltip(props: { panel: string; title: string; at: Point }) {
  const [usage] = createResource(
    () => props.panel,
    (panel) =>
      ipc
        .usagePanelTotals(panel, sessionStart)
        .then((totals) => (totals.requests > 0 ? badgeText(totals) : null))
        .catch(() => null),
  );
  const port = () => portOf(props.panel);
  return (
    <div
      class="af-stage-tooltip"
      role="tooltip"
      style={{ left: `${props.at.x}px`, top: `${props.at.y}px` }}
    >
      <strong>{props.title}</strong>
      <span>
        {t('stage.tooltip.tool')}: {port()?.label() ?? '—'}
      </span>
      <Show when={port()?.kind === 'api'}>
        <span>
          {t('stage.tooltip.model')}: {port()?.label()}
        </span>
      </Show>
      <span>
        {t('stage.tooltip.status')}: {statusOf(props.panel)}
      </span>
      <span>
        {t('stage.tooltip.usage')}: {usage() ?? '—'}
      </span>
    </div>
  );
}

export function StageOverlay(props: {
  /** Panel ids (stable, so bubbles keep their typewriter state while characters move). */
  ids: readonly string[];
  character: (id: string) => OverlayCharacter | undefined;
  labels: readonly (ClusterLabel & { at: Point })[];
  size: number;
  now: number;
  hover: { panel: string; title: string; at: Point } | null;
}) {
  return (
    <div class="af-stage-overlay" aria-hidden="false">
      <For each={props.labels}>
        {(label) => (
          <div
            class="af-stage-label"
            style={{
              left: `${label.at.x}px`,
              top: `${label.at.y}px`,
              '--af-ws-color': label.color,
            }}
          >
            {label.name}
          </div>
        )}
      </For>
      <For each={props.ids}>
        {(id) => {
          const bubble = () => bubbleView(activityOf(id), props.now);
          const at = () => props.character(id)?.at ?? { x: -9999, y: -9999 };
          return (
            <>
              <div
                class="af-stage-name"
                style={{ left: `${at().x}px`, top: `${at().y + props.size / 2 + 4}px` }}
              >
                {props.character(id)?.name}
              </div>
              <div
                class="af-stage-bubble"
                classList={{ hidden: !bubble().visible }}
                title={bubble().full}
                style={{ left: `${at().x}px`, top: `${at().y - props.size / 2 - 6}px` }}
              >
                <Show when={bubble().visible}>
                  <SpeechBubble
                    text={bubble().text}
                    typewriter={settings.characters.typewriter}
                    speed={settings.characters.speed}
                  />
                </Show>
              </div>
            </>
          );
        }}
      </For>
      <Show when={props.hover} keyed>
        {(hover) => <Tooltip panel={hover.panel} title={hover.title} at={hover.at} />}
      </Show>
    </div>
  );
}
