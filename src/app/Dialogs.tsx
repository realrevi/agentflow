import { For, onCleanup, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { answerDialog, dialog, type DialogRequest } from '../shared/dialog';

function DialogBox(props: { request: DialogRequest }) {
  let box: HTMLDivElement | undefined;
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return;
    event.stopPropagation();
    answerDialog(props.request.cancelId);
  };
  onMount(() => {
    document.addEventListener('keydown', onKeyDown, true);
    const primary = box?.querySelector<HTMLButtonElement>('.af-button:not(.secondary)');
    (primary ?? box?.querySelector<HTMLButtonElement>('button'))?.focus();
  });
  onCleanup(() => document.removeEventListener('keydown', onKeyDown, true));

  return (
    <div
      class="af-dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) answerDialog(props.request.cancelId);
      }}
    >
      <div
        ref={box}
        class="af-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="af-dialog-title"
      >
        <h2 id="af-dialog-title" class="af-dialog-title">
          {props.request.title}
        </h2>
        <Show when={props.request.message}>
          <p class="af-dialog-message">{props.request.message}</p>
        </Show>
        <div class="af-dialog-actions">
          <For each={props.request.choices}>
            {(choice) => (
              <button
                type="button"
                class="af-button"
                classList={{
                  secondary: choice.kind !== 'primary' && choice.kind !== 'danger',
                  danger: choice.kind === 'danger',
                }}
                onClick={() => answerDialog(choice.id)}
              >
                {choice.label}
              </button>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}

/** Renders the open question from shared/dialog.ts, if any. */
export function Dialogs() {
  return (
    <Show when={dialog()} keyed>
      {(request) => (
        <Portal>
          <DialogBox request={request} />
        </Portal>
      )}
    </Show>
  );
}
