// Small popover menu anchored to a trigger element. Closes on outside click, Escape or select.
import { For, onCleanup, onMount, Show, type JSX } from 'solid-js';
import { Portal } from 'solid-js/web';
import { checkIcon } from './icons';
import { PixelIcon } from './PixelIcon';

export interface MenuItem {
  id: string;
  label: string;
  checked?: boolean;
  disabled?: boolean;
  /** Tooltip, e.g. why an item is disabled or a full path. */
  title?: string;
  /** Optional group heading rendered above this item. */
  heading?: string;
}

export type MenuPlacement = 'below-end' | 'above-start' | 'above-end';

interface MenuProps {
  anchor: HTMLElement;
  items: readonly MenuItem[];
  placement: MenuPlacement;
  onSelect: (id: string) => void;
  onClose: () => void;
}

/** Fixed-position style that places a popover next to `anchor`. */
export function anchorPosition(anchor: HTMLElement, placement: MenuPlacement): JSX.CSSProperties {
  const rect = anchor.getBoundingClientRect();
  const gap = 4;
  if (placement === 'below-end') {
    return { top: `${rect.bottom + gap}px`, right: `${window.innerWidth - rect.right}px` };
  }
  const bottom = `${window.innerHeight - rect.top + gap}px`;
  return placement === 'above-start'
    ? { bottom, left: `${rect.left}px` }
    : { bottom, right: `${window.innerWidth - rect.right}px` };
}

export function Menu(props: MenuProps): JSX.Element {
  let menu: HTMLDivElement | undefined;

  const onPointerDown = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!menu?.contains(target) && !props.anchor.contains(target)) props.onClose();
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') props.onClose();
  };

  onMount(() => {
    document.addEventListener('pointerdown', onPointerDown, true);
    document.addEventListener('keydown', onKeyDown);
    const checked = menu?.querySelector<HTMLButtonElement>('[aria-checked="true"]');
    (checked ?? menu?.querySelector<HTMLButtonElement>('button:not([aria-disabled])'))?.focus();
  });
  onCleanup(() => {
    document.removeEventListener('pointerdown', onPointerDown, true);
    document.removeEventListener('keydown', onKeyDown);
  });

  return (
    <Portal>
      <div
        ref={menu}
        class="af-menu"
        role="menu"
        style={anchorPosition(props.anchor, props.placement)}
      >
        <For each={props.items}>
          {(item) => (
            <>
              <Show when={item.heading}>
                <div class="af-menu-heading">{item.heading}</div>
              </Show>
              <button
                type="button"
                class="af-menu-item"
                role={item.checked === undefined ? 'menuitem' : 'menuitemradio'}
                aria-checked={item.checked}
                aria-disabled={item.disabled ? 'true' : undefined}
                title={item.title}
                onClick={() => {
                  // aria-disabled (not `disabled`) so the tooltip still shows on hover.
                  if (item.disabled) return;
                  props.onSelect(item.id);
                  props.onClose();
                }}
              >
                <span class="af-menu-check">
                  <Show when={item.checked}>
                    <PixelIcon art={checkIcon} />
                  </Show>
                </span>
                {item.label}
              </button>
            </>
          )}
        </For>
      </div>
    </Portal>
  );
}
