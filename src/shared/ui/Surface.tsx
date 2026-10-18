// Small presentational pieces: Badge, Card, EmptyState (icon, short text, one action),
// Skeleton and Tooltip. Styles: controls.css.
import { createSignal, onCleanup, Show, splitProps, type JSX } from 'solid-js';
import { Portal } from 'solid-js/web';
import { Icon, type IconName } from './Icon';

export type BadgeTone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger';

export function Badge(props: { tone?: BadgeTone; title?: string; children: JSX.Element }) {
  return (
    <span
      classList={{
        'af-badge': true,
        accent: props.tone === 'accent',
        success: props.tone === 'success',
        warning: props.tone === 'warning',
        danger: props.tone === 'danger',
      }}
      title={props.title}
    >
      {props.children}
    </span>
  );
}

export function Card(props: JSX.HTMLAttributes<HTMLDivElement>): JSX.Element {
  const [local, rest] = splitProps(props, ['class']);
  return <div class={local.class ? `af-card ${local.class}` : 'af-card'} {...rest} />;
}

export function EmptyState(props: {
  icon?: IconName;
  title?: string;
  text: string;
  action?: JSX.Element;
  class?: string;
}): JSX.Element {
  return (
    <div class={props.class ? `af-empty ${props.class}` : 'af-empty'}>
      <Show when={props.icon}>
        {(name) => <Icon name={name()} size={32} strokeWidth={1.5} class="af-empty-icon" />}
      </Show>
      <Show when={props.title}>
        <p class="af-empty-title">{props.title}</p>
      </Show>
      <p class="af-empty-text">{props.text}</p>
      {props.action}
    </div>
  );
}

export function Skeleton(props: { width?: string; height?: string; class?: string }) {
  return (
    <span
      class={props.class ? `af-skeleton ${props.class}` : 'af-skeleton'}
      style={{ width: props.width ?? '100%', height: props.height }}
      aria-hidden="true"
    />
  );
}

/** Shows `text` next to its child on hover and keyboard focus (after a short delay). */
export function Tooltip(props: {
  text: string;
  placement?: 'right' | 'top' | 'bottom';
  children: JSX.Element;
}): JSX.Element {
  const [at, setAt] = createSignal<JSX.CSSProperties | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const show = (event: Event) => {
    const target = event.currentTarget as HTMLElement;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const rect = target.getBoundingClientRect();
      const place = props.placement ?? 'right';
      setAt(
        place === 'right'
          ? { left: `${rect.right + 8}px`, top: `${rect.top + rect.height / 2 - 12}px` }
          : place === 'top'
            ? { left: `${rect.left}px`, bottom: `${window.innerHeight - rect.top + 6}px` }
            : { left: `${rect.left}px`, top: `${rect.bottom + 6}px` },
      );
    }, 350);
  };
  const hide = () => {
    clearTimeout(timer);
    setAt(null);
  };
  onCleanup(() => clearTimeout(timer));
  return (
    <span
      class="af-tooltip-anchor"
      onPointerEnter={show}
      onPointerLeave={hide}
      onFocusIn={show}
      onFocusOut={hide}
      onPointerDown={hide}
    >
      {props.children}
      <Show when={at()}>
        {(style) => (
          <Portal>
            <div class="af-tooltip" role="tooltip" style={style()}>
              {props.text}
            </div>
          </Portal>
        )}
      </Show>
    </span>
  );
}
