// A single modal question at a time (rendered by app/Dialogs.tsx). Asking while another
// dialog is open answers the older one with its cancel choice.
import { createSignal } from 'solid-js';

export interface DialogChoice {
  id: string;
  label: string;
  kind?: 'primary' | 'secondary' | 'danger';
}

export interface DialogRequest {
  title: string;
  message?: string;
  choices: readonly DialogChoice[];
  /** Returned for Escape and clicks outside the dialog. */
  cancelId: string;
}

interface OpenDialog extends DialogRequest {
  resolve(choiceId: string): void;
}

const [dialog, setDialog] = createSignal<OpenDialog | null>(null);

export { dialog };

export function askChoice(request: DialogRequest): Promise<string> {
  return new Promise((resolve) => {
    const previous = dialog();
    setDialog({ ...request, resolve });
    previous?.resolve(previous.cancelId);
  });
}

export function answerDialog(choiceId: string): void {
  const current = dialog();
  setDialog(null);
  current?.resolve(choiceId);
}
