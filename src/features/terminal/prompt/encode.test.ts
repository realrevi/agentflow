import { describe, expect, it } from 'vitest';
import { encodePrompt } from './encode';

describe('encodePrompt', () => {
  it('sends single lines as typed, then Enter', () => {
    expect(encodePrompt('fix the tests', false)).toBe('fix the tests\r');
    expect(encodePrompt('fix the tests', true)).toBe('fix the tests\r');
  });

  it('wraps multiline text in bracketed paste when the program enabled it', () => {
    expect(encodePrompt('line one\nline two', true)).toBe('\x1b[200~line one\rline two\x1b[201~\r');
    expect(encodePrompt('a\r\nb', true)).toBe('\x1b[200~a\rb\x1b[201~\r');
  });

  it('sends plain CRs without bracketed paste mode', () => {
    expect(encodePrompt('line one\nline two', false)).toBe('line one\rline two\r');
  });

  it('strips paste markers so the text cannot end the paste early', () => {
    expect(encodePrompt('a\x1b[201~\nrm -rf', true)).toBe('\x1b[200~a\rrm -rf\x1b[201~\r');
  });
});
