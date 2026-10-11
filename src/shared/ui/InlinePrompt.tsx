// A one-line input in a popover anchored like Menu. Enter submits, Escape or an outside
// click cancels.
import { onCleanup, onMount, type JSX } from 'solid-js';
import { Portal } from 'solid-js/web';
import { anchorPosition, type MenuPlacement } from './Menu';

interface InlinePromptProps {
  anchor: HTMLElement;
  placement: MenuPlacement;
  label: string;
  placeholder: string;
  submitLabel: string;
  onSubmit: (value: string) => void;
  onClose: () => void;
}

export function InlinePrompt(props: InlinePromptProps): JSX.Element {
  let box: HTMLFormElement | undefined;
  let input: HTMLInputElement | undefined;

  const onPointerDown = (event: PointerEvent) => {
    const target = event.target as Node;
    if (!box?.contains(target) && !props.anchor.contains(target)) props.onClose();
  };

  onMount(() => {
    document.addEventListener('pointerdown', onPointerDown, true);
    input?.focus();
  });
  onCleanup(() => document.removeEventListener('pointerdown', onPointerDown, true));

  const submit = (event: SubmitEvent) => {
    event.preventDefault();
    const value = input?.value.trim() ?? '';
    if (!value) return;
    props.onSubmit(value);
    props.onClose();
  };

  return (
    <Portal>
      <form
        ref={box}
        class="af-menu af-prompt"
        style={anchorPosition(props.anchor, props.placement)}
        onSubmit={submit}
        onKeyDown={(event) => event.key === 'Escape' && props.onClose()}
      >
        <input
          ref={input}
          class="af-input"
          type="text"
          spellcheck={false}
          autocomplete="off"
          aria-label={props.label}
          placeholder={props.placeholder}
        />
        <button type="submit" class="af-button small">
          {props.submitLabel}
        </button>
      </form>
    </Portal>
  );
}
