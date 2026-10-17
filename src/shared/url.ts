// Which links may leave the app: only plain web pages go to the system browser.

/** The normalized URL if `href` is an absolute http(s) URL, else `null`. */
export function webUrl(href: string): string | null {
  try {
    const url = new URL(href.trim());
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null;
  } catch {
    return null;
  }
}
