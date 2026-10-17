import { describe, expect, it } from 'vitest';
import agentRs from '../../../src-tauri/src/settings/agent.rs?raw';
import chatRs from '../../../src-tauri/src/settings/chat.rs?raw';
import modelRs from '../../../src-tauri/src/settings/model.rs?raw';
import providersRs from '../../../src-tauri/src/settings/providers.rs?raw';
import usageRs from '../../../src-tauri/src/settings/usage.rs?raw';
import { PRESETS } from '../providers/presets';
import { DEFAULT_SETTINGS } from './defaults';

// Rust owns settings.json and its migrations (src-tauri/src/settings/migrate.rs, tested
// there). The frontend's defaults mirror `Settings::default()` and must stay in step.
const RUST = {
  'model.rs': modelRs,
  'providers.rs': providersRs,
  'chat.rs': chatRs,
  'usage.rs': usageRs,
  'agent.rs': agentRs,
};
const rust = (file: keyof typeof RUST) => RUST[file];

/** The body of `impl Default for <name>` in a Rust file. */
function defaultImpl(file: keyof typeof RUST, name: string): string {
  const source = rust(file);
  const start = source.indexOf(`impl Default for ${name} {`);
  expect(start, `impl Default for ${name}`).toBeGreaterThanOrEqual(0);
  return source.slice(start, source.indexOf('\n}\n', start));
}

describe('settings defaults mirror the Rust schema', () => {
  it('uses the current settings version', () => {
    const version = /CURRENT_VERSION: u32 = (\d+);/.exec(rust('model.rs'))?.[1];
    expect(DEFAULT_SETTINGS.version).toBe(Number(version));
  });

  it('starts with no providers and the Rust chat defaults', () => {
    expect(DEFAULT_SETTINGS.providers).toEqual([]);
    const chat = defaultImpl('chat.rs', 'ChatSettings');
    const expected = DEFAULT_SETTINGS.chat;
    expect(chat).toContain(`smooth_reveal: ${expected.smoothReveal}`);
    expect(chat).toContain(`prompt_caching: ${expected.promptCaching}`);
    expect(chat).toContain(`compact_threshold: ${expected.compactThreshold}`);
    expect(chat).toContain(`compact_keep_turns: ${expected.compactKeepTurns}`);
    expect(chat).toContain(`auto_compact: ${expected.autoCompact}`);
    const window = /DEFAULT_CONTEXT_WINDOW: u32 = ([\d_]+);/.exec(rust('chat.rs'))?.[1];
    expect(expected.defaultContextWindow).toBe(Number(window?.replaceAll('_', '')));
    expect(expected.contextWindows).toEqual([]);
    expect(expected.compactProviderId).toBeNull();
    expect(expected.compactModel).toBe('');
  });

  it('tracks CLI usage by default, with no prices or budgets', () => {
    const usage = defaultImpl('usage.rs', 'UsageSettings');
    expect(usage).toContain(`track_cli: ${DEFAULT_SETTINGS.usage.trackCli}`);
    expect(usage).toContain(`cli_subscription: ${DEFAULT_SETTINGS.usage.cliSubscription}`);
    expect(DEFAULT_SETTINGS.usage.freeProviders).toEqual([]);
    expect(DEFAULT_SETTINGS.usage.prices).toEqual([]);
    expect(DEFAULT_SETTINGS.usage.budgets).toEqual({
      daily: null,
      monthly: null,
      providers: [],
      hardStop: false,
    });
  });

  it('uses the Rust agent defaults', () => {
    const agent = defaultImpl('agent.rs', 'AgentSettings');
    const expected = DEFAULT_SETTINGS.agent;
    const num = (field: string) =>
      Number(new RegExp(`${field}: ([0-9_]+),`).exec(agent)?.[1]?.replaceAll('_', ''));
    expect(num('max_iterations')).toBe(expected.maxIterations);
    expect(num('tool_result_cap')).toBe(expected.toolResultCap);
    expect(num('command_timeout_secs')).toBe(expected.commandTimeoutSecs);
    expect(agent).toContain(`default_prompt: ${expected.defaultPrompt}`);
    expect(agent).toContain('default_mode: PermissionMode::Ask');
    expect(expected.defaultMode).toBe('ask');
    expect(expected.toolModels).toEqual([]);
  });

  it('knows every sidebar view the backend accepts', () => {
    const views = /SIDEBAR_VIEWS: \[&str; \d+\] = \[([^\]]+)\]/.exec(rust('model.rs'))?.[1];
    expect([...(views ?? '').matchAll(/"([a-z]+)"/g)].map((m) => m[1])).toEqual([
      'agents',
      'explorer',
      'characters',
      'usage',
    ]);
  });

  it('knows every provider preset the backend accepts', () => {
    const presets = /OPENAI_PRESETS: \[&str; \d+\] = \[([^\]]+)\]/s.exec(rust('providers.rs'))?.[1];
    const names = [...(presets ?? '').matchAll(/"([a-z]+)"/g)].map((m) => m[1]);
    const openai = PRESETS.filter((preset) => preset.kind === 'openai').map((preset) => preset.id);
    expect(openai).toEqual(names);
    expect(PRESETS.filter((preset) => preset.kind === 'anthropic').map((p) => p.id)).toEqual([
      'anthropic',
    ]);
  });
});
