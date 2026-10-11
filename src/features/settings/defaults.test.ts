import { describe, expect, it } from 'vitest';
import modelRs from '../../../src-tauri/src/settings/model.rs?raw';
import providersRs from '../../../src-tauri/src/settings/providers.rs?raw';
import { DEFAULT_SETTINGS } from './defaults';

// Rust owns settings.json and its migrations (src-tauri/src/settings/migrate.rs, tested
// there). The frontend's defaults mirror `Settings::default()` and must stay in step.
const rust = (file: 'model.rs' | 'providers.rs') => (file === 'model.rs' ? modelRs : providersRs);

describe('settings defaults mirror the Rust schema', () => {
  it('uses the current settings version', () => {
    const version = /CURRENT_VERSION: u32 = (\d+);/.exec(rust('model.rs'))?.[1];
    expect(DEFAULT_SETTINGS.version).toBe(Number(version));
  });

  it('starts v5 with no providers and smooth reveal on', () => {
    expect(DEFAULT_SETTINGS.providers).toEqual([]);
    expect(DEFAULT_SETTINGS.chat).toEqual({ smoothReveal: true });
    expect(rust('providers.rs')).toContain('smooth_reveal: true');
  });

  it('knows every provider preset the backend accepts', () => {
    const presets = /OPENAI_PRESETS: \[&str; \d+\] = \[([^\]]+)\]/s.exec(rust('providers.rs'))?.[1];
    const names = [...(presets ?? '').matchAll(/"([a-z]+)"/g)].map((m) => m[1]);
    expect(names).toEqual([
      'openai',
      'deepseek',
      'gemini',
      'openrouter',
      'groq',
      'ollama',
      'custom',
    ]);
  });
});
