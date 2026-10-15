import { describe, expect, it } from 'vitest';
import { estimateTokens, MAX_ATTACHMENT_BYTES, totalBytes, withAttachment } from './attachments';

describe('attachments', () => {
  it('estimates tokens roughly', () => {
    expect(estimateTokens('')).toBe(0);
    expect(estimateTokens('abc')).toBe(1);
    expect(estimateTokens('x'.repeat(400))).toBe(100);
  });

  it('caps the total at 200 KB and replaces a re-attached file', () => {
    const half = { path: 'a.txt', content: 'a'.repeat(MAX_ATTACHMENT_BYTES / 2) };
    let list = withAttachment([], half);
    list = withAttachment(list, { path: 'b.txt', content: 'b'.repeat(MAX_ATTACHMENT_BYTES / 2) });
    expect(totalBytes(list)).toBe(MAX_ATTACHMENT_BYTES);
    expect(() => withAttachment(list, { path: 'c.txt', content: 'c' })).toThrow();
    list = withAttachment(list, { path: 'a.txt', content: 'short' });
    expect(list.map((item) => item.path)).toEqual(['b.txt', 'a.txt']);
  });

  it('counts bytes, not characters', () => {
    expect(totalBytes([{ path: 'u.txt', content: 'ü🤖' }])).toBe(6);
  });
});
