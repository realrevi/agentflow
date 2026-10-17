import { describe, expect, it } from 'vitest';
import { webUrl } from './url';

describe('webUrl', () => {
  it('accepts absolute http and https links', () => {
    expect(webUrl('https://example.com/a?b=1')).toBe('https://example.com/a?b=1');
    expect(webUrl(' http://localhost:3000 ')).toBe('http://localhost:3000/');
  });

  it('rejects other schemes and relative links', () => {
    for (const href of ['javascript:alert(1)', 'file:///etc/passwd', 'mailto:a@b.c', '/docs', '#x', '']) {
      expect(webUrl(href), href).toBeNull();
    }
  });
});
