import { describe, expect, it } from 'vitest';
import { OutputDecoder } from './decoder';

const bytes = (text: string) => new TextEncoder().encode(text);

describe('OutputDecoder', () => {
  it('joins a 2-byte character split across chunks', () => {
    const decoder = new OutputDecoder();
    const [a, b] = [0xc3, 0xa9]; // "é"
    expect(decoder.decode(new Uint8Array([0x63, 0x61, 0x66, a]))).toBe('caf');
    expect(decoder.decode(new Uint8Array([b]))).toBe('é');
  });

  it('joins a 4-byte emoji split into three chunks', () => {
    const decoder = new OutputDecoder();
    const emoji = bytes('🤖'); // f0 9f a4 96
    expect(decoder.decode(emoji.slice(0, 1))).toBe('');
    expect(decoder.decode(emoji.slice(1, 3))).toBe('');
    expect(decoder.decode(emoji.slice(3))).toBe('🤖');
  });

  it('decodes mixed scripts chunk by chunk', () => {
    const decoder = new OutputDecoder();
    const all = bytes('Ajan 1 → 智能体 · Агент');
    let out = '';
    for (let i = 0; i < all.length; i += 3) out += decoder.decode(all.slice(i, i + 3));
    expect(out + decoder.flush()).toBe('Ajan 1 → 智能体 · Агент');
  });

  it('replaces invalid and unfinished bytes', () => {
    const decoder = new OutputDecoder();
    expect(decoder.decode(new Uint8Array([0x61, 0xff, 0x62]))).toBe('a�b');
    expect(decoder.decode(new Uint8Array([0xe2, 0x82]))).toBe('');
    expect(decoder.flush()).toBe('�');
  });
});
