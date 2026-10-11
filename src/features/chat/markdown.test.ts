// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { renderMarkdown, sanitizeHtml } from './markdown';

function dom(html: string): HTMLElement {
  const host = document.createElement('div');
  host.innerHTML = html;
  return host;
}

describe('markdown rendering', () => {
  it('renders GFM with the agent-bold style and copyable code blocks', () => {
    const host = dom(renderMarkdown('**Done!**\n\n```ts\nconst a = 1 < 2;\n```\n\n- [x] task'));
    expect(host.querySelector('strong.af-agent-bold')?.textContent).toBe('Done!');
    expect(host.querySelector('.af-code-lang')?.textContent).toBe('ts');
    expect(host.querySelector('pre code')?.textContent).toBe('const a = 1 < 2;');
    expect(host.querySelector('button.af-code-copy')).not.toBeNull();
    const checkbox = host.querySelector('input[type="checkbox"]');
    expect(checkbox?.hasAttribute('disabled')).toBe(true);
  });

  it('shows raw HTML from the model as text, never as markup', () => {
    const host = dom(renderMarkdown('Hi <script>alert(1)</script> <img src=x onerror="alert(2)">'));
    expect(host.querySelector('script, img')).toBeNull();
    expect(host.textContent).toContain('<script>alert(1)</script>');
  });

  it('drops dangerous links and turns images into links', () => {
    const host = dom(
      renderMarkdown('[a](javascript:alert(1)) [b](https://ok.example) ![c](https://x/y.png)'),
    );
    const hrefs = [...host.querySelectorAll('a')].map((a) => a.getAttribute('href'));
    expect(hrefs).not.toContain('javascript:alert(1)');
    expect(hrefs).toContain('https://ok.example');
    expect(hrefs).toContain('https://x/y.png');
    expect(host.querySelector('img')).toBeNull();
    expect(host.querySelector('a')?.getAttribute('rel')).toBe('noopener noreferrer');
  });
});

describe('sanitizer', () => {
  it('strips scripts, event handlers, frames and unsafe URLs', () => {
    const dirty =
      '<p onclick="steal()" onmouseover="x()">hi</p><script>bad()</script>' +
      '<a href="javascript:evil()">l</a><iframe src="https://x"></iframe>' +
      '<svg><script>alert(1)</script></svg><style>*{display:none}</style>' +
      '<button onclick="x()">fake</button><form><input type="text"></form>' +
      '<a href="data:text/html;base64,PHNjcmlwdD4=">d</a>';
    const html = sanitizeHtml(dirty);
    expect(html).not.toMatch(
      /onclick|onmouseover|<script|<iframe|<svg|<style|javascript:|data:text|<form/i,
    );
    expect(html).not.toContain('fake');
    expect(html).toContain('<p>hi</p>');
  });
});
