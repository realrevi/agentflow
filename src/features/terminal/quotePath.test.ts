import { describe, expect, it } from 'vitest';
import { quotePath } from './quotePath';

describe('quotePath', () => {
  it('single-quotes for POSIX shells', () => {
    expect(quotePath('src/main.rs', false)).toBe("'src/main.rs'");
    expect(quotePath("it's here/a b.txt", false)).toBe("'it'\\''s here/a b.txt'");
  });

  it('double-quotes with backslashes on Windows', () => {
    expect(quotePath('src/my file.ts', true)).toBe('"src\\my file.ts"');
  });
});
