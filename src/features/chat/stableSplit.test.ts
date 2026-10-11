import { describe, expect, it } from 'vitest';
import { splitStable } from './stableSplit';

describe('splitStable', () => {
  it('splits after the last blank line', () => {
    expect(splitStable('# Title\n\nFirst para.\n\nSecond')).toEqual({
      head: '# Title\n\nFirst para.\n\n',
      tail: 'Second',
    });
    expect(splitStable('just one line')).toEqual({ head: '', tail: 'just one line' });
  });

  it('never splits inside a code fence', () => {
    const open = 'Intro\n\n```ts\nconst a = 1;\n\nconst b = 2;\n';
    expect(splitStable(open)).toEqual({
      head: 'Intro\n\n',
      tail: '```ts\nconst a = 1;\n\nconst b = 2;\n',
    });
    const closed = `${open}\`\`\`\n\nAfter`;
    expect(splitStable(closed).tail).toBe('After');
    const longer = 'a\n\n````\n```\n\nstill code\n';
    expect(splitStable(longer).head).toBe('a\n\n');
  });

  it('head + tail is always the whole text', () => {
    const text = 'x\n\n~~~\ny\n~~~\n\n- a\n- b\n\nend';
    for (let i = 0; i <= text.length; i++) {
      const { head, tail } = splitStable(text.slice(0, i));
      expect(head + tail).toBe(text.slice(0, i));
    }
  });
});
