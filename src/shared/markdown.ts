// Markdown for assistant replies and library items: marked (GFM, line breaks) with raw
// HTML shown as text, then DOMPurify with a small allow-list as a second line of defense.
// Code blocks get a copy button when a label is given (the chat handles it by delegation);
// bold uses the agent-bold style. Only lazy panels import this (marked + DOMPurify).
import './markdown.css';
import DOMPurify from 'dompurify';
import { Marked } from 'marked';

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

/** The copy button's label for the current render (none = no button). */
let copyLabel: string | null = null;

const marked = new Marked({ gfm: true, breaks: true, async: false });
marked.use({
  renderer: {
    // Model output never becomes live HTML: tags are shown as typed.
    html({ text }) {
      return escapeHtml(text);
    },
    code({ text, lang }) {
      const language = (lang ?? '').trim().split(/\s+/)[0] ?? '';
      const label = copyLabel === null ? null : escapeHtml(copyLabel);
      const button =
        label === null
          ? ''
          : `<button type="button" class="af-code-copy" data-copy="code" aria-label="${label}" title="${label}">${label}</button>`;
      return (
        `<div class="af-code"><div class="af-code-bar"><span class="af-code-lang">${escapeHtml(language)}</span>` +
        `${button}</div><pre><code>${escapeHtml(text)}</code></pre></div>`
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

/** Safe HTML for `text`; code blocks get a copy button labelled `options.copyLabel`. */
export function renderMarkdown(text: string, options: { copyLabel?: string } = {}): string {
  copyLabel = options.copyLabel ?? null;
  try {
    return sanitizeHtml(marked.parse(text, { async: false }));
  } finally {
    copyLabel = null;
  }
}
