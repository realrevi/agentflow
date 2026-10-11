// The multiline prompt bar under an agent terminal. Enter sends (written to the pty at
// once; the animation is decoration only), Shift+Enter adds a line, Up/Down recall earlier
// prompts. The panel's character walks along the text, following the caret.
import './prompt.css';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import { Avatar, reducedMotion, startAnimation, type CharacterRef } from '../../characters';
import { t } from '../../i18n';
import { settings } from '../../settings/settingsStore';
import { caretCell, caretPixel } from './caret';
import type { PromptHistory } from './history';
import { FRESH, newer, older, type Recall } from './recall';

const MAX_LINES = 6;
const WALKER_SCALE = 2;
const STOP_WALKING_MS = 600;

/** What the bar looked like when a prompt was sent, for the packet animation. */
export interface SentPrompt {
  text: string;
  textBox: DOMRect;
  textStyle: Partial<CSSStyleDeclaration>;
  origin: { x: number; y: number };
}

export interface PromptBarHandle {
  insert(text: string): void;
  focus(): void;
}

interface PromptBarProps {
  history: PromptHistory;
  character: () => CharacterRef | null;
  enabled: () => boolean;
  /** Writes the prompt to the pty; false when it could not be sent. */
  onSend: (text: string) => boolean;
  onSent: (sent: SentPrompt) => void;
  ref: (handle: PromptBarHandle) => void;
  /** Shown while the bar is disabled (default: "start the agent"). */
  disabledPlaceholder?: string;
}

export function PromptBar(props: PromptBarProps) {
  let root: HTMLDivElement | undefined;
  let input: HTMLTextAreaElement | undefined;
  let recall: Recall = FRESH;
  const [walking, setWalking] = createSignal(false);
  const [walker, setWalker] = createSignal({ x: 0, y: 0, flip: false });
  let target = { x: 0, y: 0 };
  let stopWalking: number | undefined;
  let stopMove: (() => void) | undefined;
  const size = () => 16 * WALKER_SCALE;

  /** Monospace metrics of the textarea, measured once per font size. */
  let measured = { font: '', charWidth: 8 };
  const metrics = () => {
    const style = input ? getComputedStyle(input) : undefined;
    const font = style ? `${style.fontSize} ${style.fontFamily}` : '';
    if (style && font !== measured.font) {
      const ctx = document.createElement('canvas').getContext('2d');
      if (ctx) ctx.font = font;
      measured = { font, charWidth: ctx?.measureText('M').width || measured.charWidth };
    }
    const px = (value: string | undefined) => Number.parseFloat(value ?? '0') || 0;
    return {
      charWidth: measured.charWidth,
      lineHeight: px(style?.lineHeight) || measured.charWidth * 2,
      paddingLeft: px(style?.paddingLeft),
      paddingTop: px(style?.paddingTop),
      scrollTop: input?.scrollTop ?? 0,
      contentWidth: (input?.clientWidth ?? 0) - px(style?.paddingLeft) - px(style?.paddingRight),
    };
  };

  const caretPoint = () => {
    if (!input) return { x: 0, y: 0 };
    const m = metrics();
    const columns = Math.max(1, Math.floor(m.contentWidth / m.charWidth));
    const cell = caretCell(input.value, input.selectionStart, columns);
    const pixel = caretPixel(cell, m);
    return { x: input.offsetLeft + pixel.x, y: input.offsetTop + pixel.y };
  };

  /** Moves the walker towards the caret (shared animation loop, only while it moves). */
  const follow = () => {
    if (!props.character() || !root) return;
    const caret = caretPoint();
    const maxX = Math.max(0, root.clientWidth - size());
    target = { x: Math.min(maxX, Math.max(0, caret.x - size() / 2)), y: caret.y - size() };
    if (reducedMotion()) {
      setWalker((w) => ({ ...w, x: target.x, y: target.y }));
      return;
    }
    stopMove ??= startAnimation({
      step: () => {
        const current = walker();
        const dx = target.x - current.x;
        const done = Math.abs(dx) < 0.5;
        setWalker({
          x: done ? target.x : current.x + dx * 0.25,
          y: target.y,
          flip: Math.abs(dx) >= 0.5 ? dx < 0 : current.flip,
        });
        if (done) stopMove = undefined;
        return !done;
      },
    });
  };

  const typed = () => {
    setWalking(true);
    window.clearTimeout(stopWalking);
    stopWalking = window.setTimeout(() => setWalking(false), STOP_WALKING_MS);
    follow();
  };

  /** Grows with the text up to MAX_LINES, then scrolls. */
  const grow = () => {
    if (!input) return;
    const m = metrics();
    input.style.height = 'auto';
    const border = input.offsetHeight - input.clientHeight;
    const max = m.lineHeight * MAX_LINES + m.paddingTop * 2 + border;
    input.style.height = `${Math.min(max, input.scrollHeight + border)}px`;
  };

  const setText = (text: string) => {
    if (!input) return;
    input.value = text;
    input.setSelectionRange(text.length, text.length);
    grow();
    follow();
  };

  const send = () => {
    if (!input) return;
    const text = input.value;
    if (!text.trim() || !props.enabled()) return;
    const style = getComputedStyle(input);
    const textBox = input.getBoundingClientRect();
    const caret = caretPoint();
    const rootBox = root?.getBoundingClientRect();
    if (!props.onSend(text)) return; // written to the pty before anything else happens
    recall = FRESH;
    setText('');
    props.onSent({
      text,
      textBox,
      textStyle: {
        font: style.font,
        lineHeight: style.lineHeight,
        padding: style.padding,
        color: style.color,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all',
      },
      origin: { x: (rootBox?.left ?? 0) + caret.x, y: (rootBox?.top ?? 0) + caret.y },
    });
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (!input || event.isComposing) return;
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      send();
      return;
    }
    const texts = props.history.entries().map((entry) => entry.text);
    const atStart = input.selectionStart === 0 && input.selectionEnd === 0;
    if (event.key === 'ArrowUp' && (input.value === '' || recall.index !== null || atStart)) {
      const step = older(texts, recall, input.value);
      if (!step) return;
      event.preventDefault();
      recall = step.recall;
      setText(step.text);
    } else if (event.key === 'ArrowDown' && recall.index !== null) {
      const step = newer(texts, recall);
      if (!step) return;
      event.preventDefault();
      recall = step.recall;
      setText(step.text);
    }
  };

  props.ref({
    insert(text) {
      setText(text);
      input?.focus();
    },
    focus: () => input?.focus(),
  });

  onMount(() => {
    grow();
    follow();
    if (!root) return;
    const observer = new ResizeObserver(() => follow());
    observer.observe(root);
    onCleanup(() => observer.disconnect());
  });
  onCleanup(() => {
    window.clearTimeout(stopWalking);
    stopMove?.();
  });

  return (
    <div ref={root} class="af-prompt" style={{ 'font-size': `${settings.terminal.fontSize}px` }}>
      <Show when={props.character()}>
        {(character) => (
          <div
            class="af-prompt-walker"
            classList={{ flip: walker().flip }}
            style={{ transform: `translate(${walker().x}px, ${walker().y}px)` }}
            aria-hidden="true"
          >
            <Avatar
              character={character()}
              state={walking() ? 'walk' : 'idle'}
              scale={WALKER_SCALE}
              speed={settings.characters.speed}
            />
          </div>
        )}
      </Show>
      <textarea
        ref={input}
        class="af-prompt-input"
        rows={1}
        spellcheck={false}
        autocomplete="off"
        disabled={!props.enabled()}
        placeholder={
          props.enabled()
            ? t('prompt.placeholder')
            : (props.disabledPlaceholder ?? t('prompt.notRunning'))
        }
        aria-label={t('prompt.label')}
        onInput={() => {
          recall = recall.index === null ? recall : { ...recall, index: null };
          grow();
          typed();
        }}
        onKeyDown={onKeyDown}
        onKeyUp={follow}
        onClick={follow}
        onScroll={follow}
      />
    </div>
  );
}
