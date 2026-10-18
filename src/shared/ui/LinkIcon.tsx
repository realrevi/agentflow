// The brand icon of a link's site (Simple Icons subset), or a generic link icon.
import { Show, type JSX } from 'solid-js';
import { siteForUrl } from '../linkSites';
import { Icon } from './Icon';

export function LinkIcon(props: { url: string; size?: number }): JSX.Element {
  const site = () => siteForUrl(props.url);
  const size = () => props.size ?? 14;
  return (
    <Show when={site()} fallback={<Icon name="link" size={size()} />}>
      {(known) => (
        <svg
          class="af-icon af-link-icon"
          width={size()}
          height={size()}
          viewBox="0 0 24 24"
          role="img"
          aria-label={known().title}
          fill={known().color ?? 'currentColor'}
        >
          <path d={known().path} />
        </svg>
      )}
    </Show>
  );
}
