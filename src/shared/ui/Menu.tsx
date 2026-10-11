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

export type MenuPlacement = 'below-start' | 'below-end' | 'above-start' | 'above-end';
/** A trigger element, or a point (context menus open at the mouse). */
export type MenuAnchor = HTMLElement | { x: number; y: number };

interface MenuProps {
  anchor: MenuAnchor;
  items: readonly MenuItem[];
  placement: MenuPlacement;
  onSelect: (id: string) => void;
  onClose: () => void;
}

/** Fixed-position style that places a popover next to `anchor`. */
export function anchorPosition(anchor: MenuAnchor, placement: MenuPlacement): JSX.CSSProperties {
  if (!(anchor instanceof HTMLElement)) return pointPosition(anchor.x, anchor.y);
  const rect = anchor.getBoundingClientRect();
  const gap = 4;
  if (placement === 'below-start') return { top: `${rect.bottom + gap}px`, left: `${rect.left}px` };
  if (placement === 'below-end') {
    return { top: `${rect.bottom + gap}px`, right: `${window.innerWidth - rect.right}px` };
  }
  const bottom = `${window.innerHeight - rect.top + gap}px`;
  return placement === 'above-start'
    ? { bottom, left: `${rect.left}px` }
    : { bottom, right: `${window.innerWidth - rect.right}px` };
}

/** Opens down-right of the point, flipping near the bottom and right edges. */
function pointPosition(x: number, y: number): JSX.CSSProperties {
  const vertical =
    y > window.innerHeight * 0.6 ? { bottom: `${window.innerHeight - y}px` } : { top: `${y}px` };
  const horizontal =
    x > window.innerWidth - 220 ? { right: `${window.innerWidth - x}px` } : { left: `${x}px` };
  return { ...vertical, ...horizontal };
}

function isInside(anchor: MenuAnchor, target: Node): boolean {
  return anchor instanceof HTMLElement && anchor.contains(target);
}

export function Menu(props: MenuProps): JSX.Element {
  let menu: HTMLDivElement | undefined;

  const onPointerDown = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!menu?.contains(target) && !isInside(props.anchor, target)) props.onClose();
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
