// Inline name editor for create/rename rows. Enter commits, Escape cancels, blur commits
// a changed non-empty name.
import { onMount } from 'solid-js';

export function NameInput(props: {
  initial: string;
  label: string;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  let input: HTMLInputElement | undefined;
  let done = false;
  const finish = (commit: boolean) => {
    if (done) return;
    done = true;
    const name = input?.value.trim() ?? '';
    if (commit && name && name !== props.initial) props.onCommit(name);
    else props.onCancel();
  };

  onMount(() => {
    if (!input) return;
    input.focus();
    // Select the name without its extension, like VS Code.
    const dot = props.initial.lastIndexOf('.');
    input.setSelectionRange(0, dot > 0 ? dot : props.initial.length);
  });

  return (
    <input
      ref={input}
      class="af-tree-input"
      value={props.initial}
      spellcheck={false}
      autocomplete="off"
      aria-label={props.label}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === 'Enter') finish(true);
        else if (event.key === 'Escape') finish(false);
      }}
      onBlur={() => finish(true)}
      onClick={(event) => event.stopPropagation()}
    />
  );
}
