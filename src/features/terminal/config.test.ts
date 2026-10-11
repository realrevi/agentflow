import { describe, expect, it } from 'vitest';
import {
  agentName,
  hasAgentUi,
  parseTerminalConfig,
  terminalTitle,
  type TerminalConfig,
} from './config';

const base = { character: null, historyKey: null };

describe('parseTerminalConfig', () => {
  it('accepts a valid persisted config', () => {
    expect(
      parseTerminalConfig({
        kind: 'claude',
        command: null,
        args: ['--x'],
        cwd: '/src',
        character: { id: 'fox', palette: 1 },
        historyKey: 'abc-123',
      }),
    ).toEqual({
      kind: 'claude',
      command: null,
      args: ['--x'],
      cwd: '/src',
      character: { id: 'fox', palette: 1 },
      historyKey: 'abc-123',
    });
  });

  it('repairs or rejects bad data', () => {
    expect(parseTerminalConfig({ kind: 'shell', args: [1, 'a'], cwd: '' })).toEqual({
      kind: 'shell',
      command: null,
      args: ['a'],
      cwd: null,
      ...base,
    });
    expect(
      parseTerminalConfig({
        kind: 'codex',
        character: { id: 'fox', palette: -1 },
        historyKey: '../x',
      }),
    ).toMatchObject(base);
    expect(parseTerminalConfig({ kind: 'custom', command: '  ' })).toBeNull();
    expect(parseTerminalConfig({ kind: 'bash' })).toBeNull();
    expect(parseTerminalConfig('shell')).toBeNull();
  });

  it('phase 3 configs (no character yet) still parse', () => {
    expect(parseTerminalConfig({ kind: 'gemini', command: null, args: [], cwd: null })).toEqual({
      kind: 'gemini',
      command: null,
      args: [],
      cwd: null,
      ...base,
    });
  });

  it('only shells lack the agent UI', () => {
    expect(hasAgentUi('shell')).toBe(false);
    expect(hasAgentUi('custom')).toBe(true);
    expect(hasAgentUi('claude')).toBe(true);
  });
});

describe('terminalTitle', () => {
  it('uses the command line for custom terminals, shortened', () => {
    const long = 'npm run dev -- --host 0.0.0.0 --port 5173 --open';
    const custom = (command: string): TerminalConfig => ({
      kind: 'custom',
      command,
      args: [],
      cwd: null,
      ...base,
    });
    const title = terminalTitle(custom(long));
    expect(title.length).toBeLessThanOrEqual(32);
    expect(title.endsWith('…')).toBe(true);
    expect(terminalTitle(custom('htop'))).toBe('htop');
  });

  it('names the agent after the program a custom command runs', () => {
    const custom = (command: string): TerminalConfig => ({
      kind: 'custom',
      command,
      args: [],
      cwd: null,
      ...base,
    });
    expect(agentName(custom('/usr/local/bin/aider --model x'))).toBe('aider');
    expect(agentName(custom('C:\\tools\\agent.exe'))).toBe('agent.exe');
  });
});
