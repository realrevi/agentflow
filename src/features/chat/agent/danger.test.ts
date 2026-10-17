import { describe, expect, it } from 'vitest';
import dangerousRs from '../../../../src-tauri/src/agent/dangerous.rs?raw';
import { DANGER_REASONS, dangerText } from './danger';

describe('dangerous command reasons', () => {
  it('has a localized text for every reason id the backend uses', () => {
    const start = dangerousRs.indexOf('const PATTERNS');
    const table = dangerousRs.slice(start, dangerousRs.indexOf('\n];', start));
    // Each entry ends with its reason id: `"recursiveDelete"` before the closing `)`.
    const ids = new Set([...table.matchAll(/"([a-zA-Z]+)",?\s*\)/g)].map((m) => m[1] ?? ''));
    expect(ids.size).toBe(Object.keys(DANGER_REASONS).length);
    for (const id of ids) expect(DANGER_REASONS, id).toHaveProperty(id);
    expect(dangerText('recursiveDelete')).toBe('recursive delete');
    expect(dangerText('unknownId')).toBe('unknownId');
  });
});
