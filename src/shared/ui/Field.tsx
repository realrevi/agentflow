// Form fields: Input, Textarea, Select, Checkbox and Toggle (a switch-styled checkbox).
// Thin wrappers that apply the shared classes; every other prop passes through.
import { splitProps, type JSX } from 'solid-js';

type Classes = { class?: string; classList?: Record<string, boolean | undefined> };

function classes(base: string, local: Classes): Record<string, boolean | undefined> {
  return { [base]: true, ...(local.class ? { [local.class]: true } : {}), ...local.classList };
}

export interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  ref?: HTMLInputElement | ((el: HTMLInputElement) => void);
}

export function Input(props: InputProps): JSX.Element {
  const [local, rest] = splitProps(props, ['class', 'classList']);
  return <input classList={classes('af-input', local)} {...rest} />;
}

export interface TextareaProps extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  ref?: HTMLTextAreaElement | ((el: HTMLTextAreaElement) => void);
}

export function Textarea(props: TextareaProps): JSX.Element {
  const [local, rest] = splitProps(props, ['class', 'classList']);
  return <textarea classList={classes('af-input', local)} {...rest} />;
}

export interface SelectProps extends JSX.SelectHTMLAttributes<HTMLSelectElement> {
  ref?: HTMLSelectElement | ((el: HTMLSelectElement) => void);
}

export function Select(props: SelectProps): JSX.Element {
  const [local, rest] = splitProps(props, ['class', 'classList']);
  return <select classList={classes('af-select', local)} {...rest} />;
}

export type CheckboxProps = Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'type'>;

export function Checkbox(props: CheckboxProps): JSX.Element {
  const [local, rest] = splitProps(props, ['class', 'classList']);
  return <input type="checkbox" classList={classes('af-checkbox', local)} {...rest} />;
}

/** On/off switch; a checkbox with role="switch" so screen readers say on/off. */
export function Toggle(props: CheckboxProps): JSX.Element {
  const [local, rest] = splitProps(props, ['class', 'classList']);
  return <input type="checkbox" role="switch" classList={classes('af-switch', local)} {...rest} />;
}
