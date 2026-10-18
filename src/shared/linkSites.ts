// Which site a link points to, for its brand icon (data: linkSiteData.ts). Subdomains
// match their site (gist.github.com -> GitHub); unknown hosts get the generic link icon.
import { LINK_SITES, type LinkSite } from './linkSiteData';
import { LUCIDE } from './ui/lucide';

export type { LinkSite };

const BY_HOST = new Map<string, LinkSite>();
for (const site of LINK_SITES) for (const host of site.hosts) BY_HOST.set(host, site);

/** The site of an absolute http(s) URL, or null (unknown, relative or another scheme). */
export function siteForUrl(href: string): LinkSite | null {
  let host: string;
  try {
    const url = new URL(href);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    host = url.hostname.toLowerCase().replace(/^www\./, '');
  } catch {
    return null;
  }
  // Longest suffix first: docs.google.com before google.com, hub.docker.com before docker.com.
  for (let candidate = host; candidate.includes('.');) {
    const site = BY_HOST.get(candidate);
    if (site) return site;
    candidate = candidate.slice(candidate.indexOf('.') + 1);
  }
  return null;
}

let injected = false;

/**
 * Adds one stylesheet that draws each site's icon before `a[data-site]` links (used by the
 * Markdown renderer, which only sets the attribute). Idempotent.
 */
export function ensureLinkIconStyles(): void {
  if (injected || typeof document === 'undefined') return;
  injected = true;
  const rules = LINK_SITES.map((site) => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="${site.path}"/></svg>`;
    const mask = `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
    const color = site.color ?? 'currentColor';
    return `a[data-site='${site.id}']::before{-webkit-mask-image:${mask};mask-image:${mask};background-color:${color}}`;
  });
  const generic = LUCIDE.link.map(([tag, attrs]) => {
    const list = Object.entries(attrs)
      .map(([key, value]) => `${key}="${value}"`)
      .join(' ');
    return `<${tag} ${list}/>`;
  });
  const linkSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${generic.join('')}</svg>`;
  const linkMask = `url("data:image/svg+xml,${encodeURIComponent(linkSvg)}")`;
  rules.push(
    `a[data-site='link']::before{-webkit-mask-image:${linkMask};mask-image:${linkMask};background-color:currentColor;opacity:.7}`,
  );
  const style = document.createElement('style');
  style.dataset['af'] = 'link-icons';
  style.textContent = rules.join('\n');
  document.head.append(style);
}
