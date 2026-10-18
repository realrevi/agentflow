// A colored file or folder icon (Material Icon Theme subset), 16 px by default.
import type { JSX } from 'solid-js';
import { fileIconUrl } from './fileIconNames';

export function FileIcon(props: {
  name: string;
  isDir?: boolean;
  expanded?: boolean;
  size?: number;
  class?: string;
}): JSX.Element {
  return (
    <img
      class={props.class ? `af-file-icon ${props.class}` : 'af-file-icon'}
      src={fileIconUrl(props.name, props.isDir ?? false, props.expanded ?? false)}
      width={props.size ?? 16}
      height={props.size ?? 16}
      alt=""
      draggable={false}
    />
  );
}
