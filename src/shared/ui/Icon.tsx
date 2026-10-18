// A Lucide UI icon (stroke icons, currentColor). Sizes: 14 in dense rows, 16 by default, 20
// in the activity bar. Decorative unless `label` is given.
import type { JSX } from 'solid-js';
import { LUCIDE, type IconName } from './lucide';

export type { IconName };

const markup = new Map<IconName, string>();

function inner(name: IconName): string {
  const cached = markup.get(name);
  if (cached) return cached;
  const html = LUCIDE[name]
    .map(([tag, attrs]) => {
      const list = Object.entries(attrs)
        .map(([key, value]) => `${key}="${value}"`)
        .join(' ');
      return `<${tag} ${list}/>`;
    })
    .join('');
  markup.set(name, html);
  return html;
}

export function Icon(props: {
  name: IconName;
  size?: number;
  strokeWidth?: number;
  class?: string;
  label?: string;
}): JSX.Element {
  const size = () => props.size ?? 16;
  return (
    <svg
      class={props.class ? `af-icon ${props.class}` : 'af-icon'}
      width={size()}
      height={size()}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width={props.strokeWidth ?? 2}
      stroke-linecap="round"
      stroke-linejoin="round"
      role={props.label ? 'img' : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : 'true'}
      innerHTML={inner(props.name)}
    />
  );
}
