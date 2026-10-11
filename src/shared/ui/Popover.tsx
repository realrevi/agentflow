// A floating box anchored to a trigger element, rendered in a portal (so tabs and panels
// that clip their content cannot cut it off). Closes on outside click or Escape.
import { onCleanup, onMount, type JSX } from 'solid-js';
import { Portal } from 'solid-js/web';
import { anchorPosition, type MenuPlacement } from './Menu';

interface PopoverProps {
  anchor: HTMLElement;
  placement: MenuPlacement;
  label: string;
  onClose: () => void;
  children: JSX.Element;
}

export function Popover(props: PopoverProps): JSX.Element {
  let box: HTMLDivElement | undefined;
  const onPointerDown = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!box?.contains(target) && !props.anchor.contains(target)) props.onClose();
  };
  const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && props.onClose();
  onMount(() => {
    document.addEventListener('pointerdown', onPointerDown, true);
    document.addEventListener('keydown', onKeyDown);
    box?.querySelector<HTMLElement>('[aria-selected="true"], button')?.focus();
  });
  onCleanup(() => {
    document.removeEventListener('pointerdown', onPointerDown, true);
    document.removeEventListener('keydown', onKeyDown);
  });
  return (
    <Portal>
      <div
        ref={box}
        class="af-popover"
        role="dialog"
        aria-label={props.label}
        style={anchorPosition(props.anchor, props.placement)}
      >
        {props.children}
      </div>
    </Portal>
  );
}
