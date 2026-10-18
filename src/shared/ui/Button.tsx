// Buttons: Button (primary / secondary / ghost / danger, sm / md, optional icon, icon-only
// with a label) and IconButton for small header and row actions. Styles: controls.css.
import { Show, splitProps, type JSX } from 'solid-js';
import { Icon, type IconName } from './Icon';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: 'sm' | 'md';
  icon?: IconName;
  /** Icon-only: the accessible name (also the tooltip). */
  label?: string;
  ref?: HTMLButtonElement | ((el: HTMLButtonElement) => void);
}

export function Button(props: ButtonProps): JSX.Element {
  const [local, rest] = splitProps(props, [
    'variant',
    'size',
    'icon',
    'label',
    'class',
    'classList',
    'children',
    'type',
  ]);
  const iconOnly = () => local.children === undefined && local.label !== undefined;
  return (
    <button
      type={local.type ?? 'button'}
      classList={{
        'af-button': true,
        secondary: local.variant === 'secondary',
        ghost: local.variant === 'ghost',
        danger: local.variant === 'danger',
        small: local.size === 'sm',
        'icon-only': iconOnly(),
        ...(local.class ? { [local.class]: true } : {}),
        ...local.classList,
      }}
      aria-label={local.label}
      title={rest.title ?? local.label}
      {...rest}
    >
      <Show when={local.icon}>
        {(name) => <Icon name={name()} size={local.size === 'sm' ? 14 : 16} />}
      </Show>
      {local.children}
    </button>
  );
}

export interface IconButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconName;
  /** Required: icon-only buttons need an accessible name (also the tooltip). */
  label: string;
  size?: number;
  active?: boolean;
  ref?: HTMLButtonElement | ((el: HTMLButtonElement) => void);
}

export function IconButton(props: IconButtonProps): JSX.Element {
  const [local, rest] = splitProps(props, ['icon', 'label', 'size', 'active', 'class', 'type']);
  return (
    <button
      type={local.type ?? 'button'}
      classList={{
        'af-icon-btn': true,
        active: Boolean(local.active),
        ...(local.class ? { [local.class]: true } : {}),
      }}
      aria-label={local.label}
      title={rest.title ?? local.label}
      {...rest}
    >
      <Icon name={local.icon} size={local.size ?? 16} />
    </button>
  );
}
