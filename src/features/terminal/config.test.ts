import { describe, expect, it } from 'vitest';
import { parseTerminalConfig, terminalTitle } from './config';

describe('parseTerminalConfig', () => {
  it('accepts a valid persisted config', () => {
    expect(
      parseTerminalConfig({ kind: 'claude', command: null, args: ['--x'], cwd: '/src' }),
    ).toEqual({ kind: 'claude', command: null, args: ['--x'], cwd: '/src' });
  });

  it('repairs or rejects bad data', () => {
    expect(parseTerminalConfig({ kind: 'shell', args: [1, 'a'], cwd: '' })).toEqual({
      kind: 'shell',
      command: null,
      args: ['a'],
      cwd: null,
    });
    expect(parseTerminalConfig({ kind: 'custom', command: '  ' })).toBeNull();
    expect(parseTerminalConfig({ kind: 'bash' })).toBeNull();
    expect(parseTerminalConfig('shell')).toBeNull();
  });
});

describe('terminalTitle', () => {
  it('uses the command line for custom terminals, shortened', () => {
    const long = 'npm run dev -- --host 0.0.0.0 --port 5173 --open';
    const title = terminalTitle({ kind: 'custom', command: long, args: [], cwd: null });
    expect(title.length).toBeLessThanOrEqual(32);
    expect(title.endsWith('…')).toBe(true);
    expect(terminalTitle({ kind: 'custom', command: 'htop', args: [], cwd: null })).toBe('htop');
  });
});
