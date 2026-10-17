// The panel's character in its top-right corner: animated avatar, a badge with the AI name,
// state and session usage, and a speech bubble. While output streams it may ride above the
// terminal cursor (an overlay only: output is never delayed or re-rendered). Click: prompt
// history.
import './agent.css';
import { createEffect, createMemo, createSignal, on, onCleanup, Show } from 'solid-js';
import type { PromptEntry } from '../../../shared/types';
import {
  Avatar,
  reducedMotion,
  spriteFor,
  startAnimation,
  type AgentState,
  type CharacterRef,
} from '../../characters';
import { t, type I18nKey } from '../../i18n';
import { settings } from '../../settings/settingsStore';
import { avatarAboveCursor, lerp, type Box, type CursorCell } from './follow';
import { HistoryPopover } from './HistoryPopover';
import { SpeechBubble } from './SpeechBubble';

const CORNER = { right: 10, top: 8 };
const PROMPT_IN_BUBBLE_MS = 2500;
const PROMPT_PREVIEW_CHARS = 40;

const STATE_LABEL: Record<AgentState, I18nKey> = {
  idle: 'characters.state.idle',
  think: 'characters.state.think',
  work: 'characters.state.work',
  done: 'characters.state.done',
  sleep: 'characters.state.sleep',
  error: 'characters.state.error',
  wait: 'characters.state.wait',
};

const BUBBLE: Partial<Record<AgentState, I18nKey>> = {
  think: 'characters.bubble.think',
  work: 'characters.bubble.work',
  done: 'characters.bubble.done',
  sleep: 'characters.bubble.sleep',
  error: 'characters.bubble.error',
  wait: 'characters.bubble.wait',
};

export interface AgentOverlayHandle {
  /** The avatar's box (client coordinates): where sent prompts fly to. */
  target(): DOMRect | null;
  /** A prompt packet landed: play "receive". */
  receive(): void;
}

interface AgentOverlayProps {
  character: CharacterRef;
  aiName: string;
  state: AgentState;
  lastPrompt: { text: string; at: number } | null;
  history: readonly PromptEntry[];
  onPickPrompt: (text: string) => void;
  /** Session usage for the badge ("12.4k · $0.08"), once there is any. */
  usage?: string | null;
  /** What the agent does right now ("Reading src/app.ts"); replaces the state's bubble. */
  status?: string | null;
  /** Terminal panels only: the cursor the avatar rides above while output streams. */
  cursor?: () => CursorCell | null;
  onCursorChange?: (listener: () => void) => () => void;
  /** The terminal host (for cell size) and the panel body (bounds). */
  host?: () => HTMLElement | undefined;
  body?: () => HTMLElement | undefined;
  ref: (handle: AgentOverlayHandle) => void;
}

export function AgentOverlay(props: AgentOverlayProps) {
  let avatar: HTMLButtonElement | undefined;
  const [receiving, setReceiving] = createSignal(false);
  const [historyOpen, setHistoryOpen] = createSignal(false);
  const [showPrompt, setShowPrompt] = createSignal(false);
  const [offset, setOffset] = createSignal({ x: 0, y: 0, hidden: false });
  const size = () => 16 * settings.characters.scale;

  props.ref({
    target: () => avatar?.getBoundingClientRect() ?? null,
    receive: () => setReceiving(true),
  });

  const sprite = () => (receiving() ? 'receive' : spriteFor(props.state));

  // The bubble shows the start of a new prompt for a moment, then the state.
  let promptTimer: number | undefined;
  createEffect(
    on(
      () => props.lastPrompt,
      (prompt) => {
        if (!prompt) return;
        setShowPrompt(true);
        window.clearTimeout(promptTimer);
        promptTimer = window.setTimeout(() => setShowPrompt(false), PROMPT_IN_BUBBLE_MS);
      },
      { defer: true },
    ),
  );
  onCleanup(() => window.clearTimeout(promptTimer));
  const bubble = createMemo(() => {
    const prompt = props.lastPrompt;
    if (showPrompt() && prompt) {
      const line = prompt.text.replace(/\s+/g, ' ').trim();
      return line.length > PROMPT_PREVIEW_CHARS
        ? `“${line.slice(0, PROMPT_PREVIEW_CHARS - 1)}…”`
        : `“${line}”`;
    }
    if (props.status && props.state === 'work') return props.status;
    const key = BUBBLE[props.state];
    return key ? t(key) : '';
  });

  // --- Riding above the cursor while output streams -------------------------------------
  const following = () =>
    props.cursor !== undefined &&
    settings.characters.cursorFollow &&
    props.state === 'work' &&
    !reducedMotion();
  let hostBox: Box = { left: 0, top: 0, width: 0, height: 0 };
  let bounds = { width: 0, height: 0 };
  const measure = () => {
    const host = props.host?.();
    const body = props.body?.();
    if (!host || !body) return;
    hostBox = {
      left: host.offsetLeft,
      top: host.offsetTop,
      width: host.clientWidth,
      height: host.clientHeight,
    };
    bounds = { width: body.clientWidth, height: body.clientHeight };
  };
  let goal = { x: 0, y: 0, hidden: false };
  let stopMove: (() => void) | undefined;
  const moveTo = (next: { x: number; y: number; hidden: boolean }) => {
    goal = next;
    const current = offset();
    if (current.x === next.x && current.y === next.y) {
      if (current.hidden !== next.hidden) setOffset(next);
      return;
    }
    stopMove ??= startAnimation({
      step: () => {
        const current = offset();
        const x = lerp(current.x, goal.x);
        const y = lerp(current.y, goal.y);
        setOffset({ x, y, hidden: goal.hidden });
        const arrived = x === goal.x && y === goal.y;
        if (arrived) stopMove = undefined;
        return !arrived;
      },
    });
  };
  /** Offset from the corner spot to just above the cursor. */
  const track = () => {
    if (!following()) return;
    const cursor = props.cursor?.() ?? null;
    const spot = cursor && avatarAboveCursor(cursor, hostBox, bounds, size());
    if (!spot) return moveTo({ ...goal, hidden: true });
    const cornerX = bounds.width - CORNER.right - size();
    moveTo({ x: spot.x - cornerX, y: spot.y - CORNER.top, hidden: false });
  };
  createEffect(() => {
    if (!following()) {
      moveTo({ x: 0, y: 0, hidden: false }); // back to the corner after "done"
      return;
    }
    measure();
    track();
    const stopListening = props.onCursorChange?.(track) ?? (() => undefined);
    const body = props.body?.();
    const observer = new ResizeObserver(() => {
      measure();
      track();
    });
    if (body) observer.observe(body);
    onCleanup(() => {
      stopListening();
      observer.disconnect();
    });
  });
  onCleanup(() => stopMove?.());

  return (
    <div class="af-agent" style={{ top: `${CORNER.top}px`, right: `${CORNER.right}px` }}>
      <button
        ref={avatar}
        type="button"
        class="af-agent-avatar"
        classList={{ riding: offset().x !== 0 || offset().y !== 0, hidden: offset().hidden }}
        style={{ transform: `translate(${offset().x}px, ${offset().y}px)` }}
        title={t('prompt.history')}
        aria-label={t('prompt.history')}
        aria-expanded={historyOpen()}
        onClick={() => setHistoryOpen((open) => !open)}
      >
        <Avatar
          character={props.character}
          state={sprite()}
          scale={settings.characters.scale}
          speed={settings.characters.speed}
          onFinished={() => setReceiving(false)}
        />
      </button>
      <div class="af-agent-badge">
        <span class="af-agent-name">{props.aiName}</span>
        <span class={`af-agent-state ${props.state}`}>{t(STATE_LABEL[props.state])}</span>
        <Show when={props.usage}>
          {(usage) => (
            <span class="af-agent-usage" title={t('usage.badge.title')}>
              {usage()}
            </span>
          )}
        </Show>
      </div>
      <Show when={settings.characters.bubble && bubble()}>
        {(text) => (
          <SpeechBubble
            text={text()}
            typewriter={settings.characters.typewriter}
            speed={settings.characters.speed}
          />
        )}
      </Show>
      <Show when={historyOpen() && avatar}>
        {(anchor) => (
          <HistoryPopover
            entries={props.history}
            anchor={anchor()}
            onPick={props.onPickPrompt}
            onClose={() => setHistoryOpen(false)}
          />
        )}
      </Show>
    </div>
  );
}
