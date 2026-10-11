import { describe, expect, it } from 'vitest';
import { sanitizePaste } from './paste';

describe('sanitizePaste', () => {
  it('removes bracketed-paste markers that could end the paste early', () => {
    expect(sanitizePaste('ls\x1b[201~\rrm -rf ~\r')).toBe('ls\rrm -rf ~\r');
    expect(sanitizePaste('\x1b[200~x\x9b201~y')).toBe('xy');
  });

  it('keeps ordinary text and other escape sequences', () => {
    expect(sanitizePaste('echo "héllo"\n\x1b[31mred\x1b[0m')).toBe(
      'echo "héllo"\n\x1b[31mred\x1b[0m',
    );
  });
});
