// SegmentedControl (one of a few options, role="radiogroup") and Tabs (role="tablist").
// Both slide an indicator behind the active item and move with the arrow keys (roving
// focus: only the active item is in the tab order).
import { createEffect, createSignal, For, on, onCleanup, onMount, Show, type JSX } from 'solid-js';
import { Icon, type IconName } from './Icon';

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  icon?: IconName;
  title?: string;
  disabled?: boolean;
}

interface Props<T extends string> {
  options: readonly SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  /** Accessible name of the group. */
  label: string;
  class?: string;
}

/** Arrow / Home / End navigation over the enabled options; returns the next value. */
export function stepOption<T extends string>(
  options: readonly SegmentOption<T>[],
  current: T,
  key: string,
): T | null {
  const enabled = options.filter((o) => !o.disabled);
  const index = enabled.findIndex((o) => o.value === current);
  const last = enabled.length - 1;
  const next =
    key === 'ArrowRight' || key === 'ArrowDown'
      ? index >= last
        ? 0
        : index + 1
      : key === 'ArrowLeft' || key === 'ArrowUp'
        ? index <= 0
          ? last
          : index - 1
        : key === 'Home'
          ? 0
          : key === 'End'
            ? last
            : null;
  return next === null ? null : (enabled[next]?.value ?? null);
}

function Group<T extends string>(props: Props<T> & { kind: 'segmented' | 'tabs' }): JSX.Element {
  let root: HTMLDivElement | undefined;
  const [indicator, setIndicator] = createSignal<JSX.CSSProperties>({ opacity: 0 });
  const place = () => {
    const active = root?.querySelector<HTMLButtonElement>('button.active');
    if (!active) {
      setIndicator({ opacity: 0 });
      return;
    }
    setIndicator({
      width: `${active.offsetWidth}px`,
      transform: `translateX(${active.offsetLeft}px)`,
    });
  };
  onMount(() => {
    place();
    // Fonts, translations and panel widths change item sizes: follow them.
    const observer = new ResizeObserver(place);
    if (root) observer.observe(root);
    onCleanup(() => observer.disconnect());
  });
  // The value may also change from outside (store, other control): re-place after render.
  createEffect(
    on(
      () => props.value,
      () => queueMicrotask(place),
      { defer: true },
    ),
  );
  const select = (value: T) => {
    if (value !== props.value) props.onChange(value);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    const next = stepOption(props.options, props.value, event.key);
    if (next === null) return;
    event.preventDefault();
    select(next);
    queueMicrotask(() => root?.querySelector<HTMLButtonElement>('button.active')?.focus());
  };
  const tabs = props.kind === 'tabs';
  return (
    <div
      ref={root}
      class={`af-${props.kind}${props.class ? ` ${props.class}` : ''}`}
      role={tabs ? 'tablist' : 'radiogroup'}
      aria-label={props.label}
      onKeyDown={onKeyDown}
    >
      <span class={tabs ? 'af-tabs-indicator' : 'af-segmented-indicator'} style={indicator()} />
      <For each={props.options}>
        {(option) => {
          const active = () => option.value === props.value;
          return (
            <button
              type="button"
              role={tabs ? 'tab' : 'radio'}
              classList={{ active: active() }}
              aria-selected={tabs ? active() : undefined}
              aria-checked={tabs ? undefined : active()}
              tabIndex={active() ? 0 : -1}
              title={option.title}
              disabled={option.disabled}
              onClick={() => select(option.value)}
            >
              <Show when={option.icon}>{(name) => <Icon name={name()} size={14} />}</Show>
              {option.label}
            </button>
          );
        }}
      </For>
    </div>
  );
}

export function SegmentedControl<T extends string>(props: Props<T>): JSX.Element {
  return <Group {...props} kind="segmented" />;
}

export function Tabs<T extends string>(props: Props<T>): JSX.Element {
  return <Group {...props} kind="tabs" />;
}
