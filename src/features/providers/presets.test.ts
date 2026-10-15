import { describe, expect, it } from 'vitest';
import { PRESETS, baseUrlProblem } from './presets';

describe('baseUrlProblem', () => {
  it('accepts every preset base URL', () => {
    for (const preset of PRESETS) {
      if (preset.baseUrl) expect(baseUrlProblem(preset.baseUrl), preset.id).toBeNull();
    }
  });

  it('allows plain http only for this machine', () => {
    for (const local of [
      'http://localhost:11434/v1',
      'http://127.0.0.1:1234/v1',
      'http://[::1]:8080',
    ]) {
      expect(baseUrlProblem(local), local).toBeNull();
    }
    for (const remote of [
      'http://api.openai.com/v1',
      'http://192.168.1.20:11434/v1',
      'http://localhost.evil.com/v1',
      'http://ollama.localhost/v1',
    ]) {
      expect(baseUrlProblem(remote), remote).toBe('insecure');
    }
  });

  it('rejects credentials, fragments, other schemes and junk', () => {
    expect(baseUrlProblem('   ')).toBe('empty');
    for (const bad of [
      'https://user:pass@example.com',
      'https://example.com/#x',
      'ftp://example.com',
      'api.openai.com/v1',
    ]) {
      expect(baseUrlProblem(bad), bad).toBe('invalid');
    }
  });
});
