// Markdown for assistant replies: marked (GFM, line breaks) with raw HTML shown as text,
// then DOMPurify with a small allow-list as a second line of defense. Code blocks get a
// copy button (handled by delegation in the message list); bold uses the agent-bold style.
import DOMPurify from 'dompurify';
import { Marked } from 'marked';
import { t } from '../i18n';

const ESCAPES: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => ESCAPES[c] ?? c);
}

const marked = new Marked({ gfm: true, breaks: true, async: false });
marked.use({
  renderer: {
    // Model output never becomes live HTML: tags are shown as typed.
    html({ text }) {
      return escapeHtml(text);
    },
    code({ text, lang }) {
      const language = (lang ?? '').trim().split(/\s+/)[0] ?? '';
      const label = escapeHtml(t('chat.copyCode'));
      return (
        `<div class="af-code"><div class="af-code-bar"><span class="af-code-lang">${escapeHtml(language)}</span>` +
        `<button type="button" class="af-code-copy" data-copy="code" aria-label="${label}" title="${label}">${label}</button></div>` +
        `<pre><code>${escapeHtml(text)}</code></pre></div>`
      );
    },
    strong({ tokens }) {
      return `<strong class="af-agent-bold">${this.parser.parseInline(tokens)}</strong>`;
    },
    // No remote images (privacy, and the CSP blocks them anyway): show a link instead.
    image({ href, text }) {
      return `<a href="${escapeHtml(href)}">${escapeHtml(text || href)}</a>`;
    },
  },
});

const ALLOWED_TAGS = [
  'p',
  'br',
  'hr',
  'strong',
  'em',
  'del',
  's',
  'code',
  'pre',
  'blockquote',
  'ul',
  'ol',
  'li',
  'a',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'table',
  'thead',
  'tbody',
  'tr',
  'th',
  'td',
  'div',
  'span',
  'button',
  'input',
];
const ALLOWED_ATTR = [
  'href',
  'title',
  'class',
  'start',
  'align',
  'type',
  'checked',
  'disabled',
  'data-copy',
  'aria-label',
];

DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') node.setAttribute('rel', 'noopener noreferrer');
  // Task-list checkboxes are display only; buttons are only our copy buttons.
  if (node.tagName === 'INPUT') {
    if (node.getAttribute('type') !== 'checkbox') node.remove();
    else node.setAttribute('disabled', '');
  }
  if (node.tagName === 'BUTTON' && node.getAttribute('data-copy') !== 'code') node.remove();
});

/** Strips anything that could run code: scripts, event handlers, unsafe URLs, frames. */
export function sanitizeHtml(html: string): string {
  // Without a usable DOM, DOMPurify returns its input unchanged: fail closed instead.
  if (!DOMPurify.isSupported) return escapeHtml(html);
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    ALLOW_DATA_ATTR: false,
    // DOMPurify's default pattern with fewer schemes: http(s) and mailto links, plus plain
    // values (it also checks attributes such as type and data-copy against this).
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i,
  });
}

export function renderMarkdown(text: string): string {
  return sanitizeHtml(marked.parse(text, { async: false }));
}
