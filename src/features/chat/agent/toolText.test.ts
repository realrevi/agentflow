import { describe, expect, it } from 'vitest';
import type { ToolCall, ToolResult } from '../../../shared/types';
import { durationText, toolState, toolSummary, toolTarget } from './toolText';

const call = (name: string, input: Record<string, unknown>): ToolCall => ({
  id: 'c',
  name,
  input,
});
const ok = (extra: Partial<ToolResult> = {}): ToolResult => ({
  callId: 'c',
  content: '',
  isError: false,
  ...extra,
});

describe('tool card summaries', () => {
  it('says what each tool did', () => {
    const read = call('read_file', { path: 'src/app.ts', offset: 1, limit: 120 });
    expect(toolSummary(read, ok({ meta: { lines: [1, 120, 400] } }))).toBe(
      'Read src/app.ts (lines 1–120)',
    );
    expect(toolSummary(read, ok({ meta: { lines: [1, 40, 40] } }))).toBe(
      'Read src/app.ts · lines: 40',
    );
    expect(toolSummary(call('list_dir', { path: 'src' }), ok({ meta: { count: 7 } }))).toBe(
      'Listed src · entries: 7',
    );
    expect(toolSummary(call('glob', { pattern: '**/*.rs' }), ok({ meta: { count: 12 } }))).toBe(
      'Found files for **/*.rs: 12',
    );
    expect(
      toolSummary(
        call('search', { regex: 'TODO', path: 'src' }),
        ok({ meta: { count: 3, files: 2 } }),
      ),
    ).toBe('Searched for TODO · matches: 3, files: 2');
    const write = call('write_file', { path: 'notes.md', content: 'a\nb' });
    expect(toolSummary(write, ok({ meta: { count: 2, created: true } }))).toBe(
      'Created notes.md · lines: 2',
    );
    expect(toolSummary(write, ok({ meta: { count: 2, created: false } }))).toBe(
      'Wrote notes.md · lines: 2',
    );
    expect(toolSummary(call('edit_file', { path: 'a.ts' }), ok({ meta: { count: 1 } }))).toBe(
      'Edited a.ts · replacements: 1',
    );
  });

  it('adds the exit code and, from one second on, the duration', () => {
    const run = call('run_command', { command: 'npm test' });
    expect(toolSummary(run, ok({ meta: { exitCode: 0 }, durationMs: 4200 }))).toBe(
      'Ran npm test · exit 0 · 4.2s',
    );
    expect(toolSummary(run, ok({ meta: { exitCode: 1 }, isError: true, durationMs: 300 }))).toBe(
      'Ran npm test · exit 1',
    );
    expect(durationText(850)).toBe('850ms');
    expect(durationText(125_000)).toBe('2m');
  });

  it('uses the active form while running, waiting, denied or skipped', () => {
    const run = call('run_command', { command: 'npm   install\n--force' });
    expect(toolTarget(run)).toBe('npm install --force');
    expect(toolSummary(run, undefined)).toBe('Running npm install --force');
    expect(toolSummary(run, ok({ outcome: 'denied', isError: true }))).toBe(
      'Running npm install --force',
    );
    const read = call('read_file', { path: 'missing.ts' });
    expect(toolSummary(read, ok({ isError: true, content: 'no such file' }))).toBe(
      'Reading missing.ts',
    );
    expect(toolTarget(call('run_command', { command: 'x'.repeat(200) })).length).toBe(80);
  });

  it('derives the state from the result and whether it runs', () => {
    expect(toolState(undefined, true)).toBe('running');
    expect(toolState(undefined, false)).toBe('pending');
    expect(toolState(ok(), false)).toBe('done');
    expect(toolState(ok({ isError: true }), false)).toBe('error');
    expect(toolState(ok({ isError: true, outcome: 'denied' }), false)).toBe('denied');
    expect(toolState(ok({ isError: true, outcome: 'skipped' }), false)).toBe('skipped');
  });
});
