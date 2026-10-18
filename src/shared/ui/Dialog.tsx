// A modal dialog: backdrop, title, body and actions. Focus moves in on open, Tab and
// Shift+Tab stay inside, Escape closes, and focus returns to where it was on close. Enters
// and exits with a short fade + scale (instant with reduced motion).
import { onCleanup, onMount, Show, type JSX } from 'solid-js';
import { Portal } from 'solid-js/web';
import { createPresence } from './presence';

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** The focusable elements inside `root`, in tab order. */
export function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => !el.hasAttribute('inert') && el.getAttribute('aria-hidden') !== 'true',
  );
}

/** Keeps Tab inside `root`; returns true when the key was handled. */
export function trapTab(root: HTMLElement, event: KeyboardEvent): boolean {
  if (event.key !== 'Tab') return false;
  const items = focusables(root);
  if (items.length === 0) {
    event.preventDefault();
    return true;
  }
  const first = items[0];
  const last = items[items.length - 1];
  const active = document.activeElement;
  const inside = active instanceof Node && root.contains(active);
  if (event.shiftKey && (active === first || !inside)) {
    event.preventDefault();
    last?.focus();
    return true;
  }
  if (!event.shiftKey && (active === last || !inside)) {
    event.preventDefault();
    first?.focus();
    return true;
  }
  return false;
}

export interface DialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children?: JSX.Element;
  /** Buttons, right-aligned at the bottom. */
  actions?: JSX.Element;
  /** `alertdialog` for questions that need an answer. */
  role?: 'dialog' | 'alertdialog';
  /** CSS selector of the element to focus first (default: the first focusable one). */
  initialFocus?: string;
  /** Close when the backdrop is clicked (default true). */
  dismissable?: boolean;
  class?: string;
}

function DialogBox(props: DialogProps & { closing: boolean }) {
  let box: HTMLDivElement | undefined;
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const titleId = `af-dialog-${Math.random().toString(36).slice(2, 8)}`;
  const onKeyDown = (event: KeyboardEvent) => {
    if (!box) return;
    if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
      return;
    }
    trapTab(box, event);
  };
  onMount(() => {
    document.addEventListener('keydown', onKeyDown, true);
    const initial = props.initialFocus
      ? box?.querySelector<HTMLElement>(props.initialFocus)
      : undefined;
    (initial ?? (box ? focusables(box)[0] : undefined) ?? box)?.focus();
  });
  onCleanup(() => {
    document.removeEventListener('keydown', onKeyDown, true);
    if (previous?.isConnected) previous.focus();
  });
  return (
    <div
      class="af-dialog-backdrop"
      data-state={props.closing ? 'closed' : 'open'}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && props.dismissable !== false) props.onClose();
      }}
    >
      <div
        ref={box}
        class={`af-dialog${props.class ? ` ${props.class}` : ''}`}
        role={props.role ?? 'dialog'}
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <h2 id={titleId} class="af-dialog-title">
          {props.title}
        </h2>
        {props.children}
        <Show when={props.actions}>
          <div class="af-dialog-actions">{props.actions}</div>
        </Show>
      </div>
    </div>
  );
}

export function Dialog(props: DialogProps): JSX.Element {
  const presence = createPresence(() => props.open);
  return (
    <Show when={presence.mounted()}>
      <Portal>
        <DialogBox {...props} closing={presence.closing()} />
      </Portal>
    </Show>
  );
}
