// Provider presets: what the settings form prefills. Base URLs come from each provider's
// docs; the backend validates them again (HTTPS only, plain HTTP only for localhost).
import type { ProviderEntry, ProviderKind, ProviderPreset } from '../../shared/types';
import { t } from '../i18n';

export interface PresetInfo {
  id: ProviderPreset;
  kind: ProviderKind;
  /** Brand names are not translated; `null` = "Custom" (translated). */
  label: string | null;
  baseUrl: string;
  needsKey: boolean;
}

export const PRESETS: readonly PresetInfo[] = [
  {
    id: 'anthropic',
    kind: 'anthropic',
    label: 'Anthropic',
    baseUrl: 'https://api.anthropic.com',
    needsKey: true,
  },
  {
    id: 'openai',
    kind: 'openai',
    label: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    needsKey: true,
  },
  {
    id: 'deepseek',
    kind: 'openai',
    label: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    needsKey: true,
  },
  {
    id: 'gemini',
    kind: 'openai',
    label: 'Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    needsKey: true,
  },
  {
    id: 'openrouter',
    kind: 'openai',
    label: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    needsKey: true,
  },
  {
    id: 'groq',
    kind: 'openai',
    label: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    needsKey: true,
  },
  {
    id: 'ollama',
    kind: 'openai',
    label: 'Ollama',
    baseUrl: 'http://localhost:11434/v1',
    needsKey: false,
  },
  { id: 'custom', kind: 'openai', label: null, baseUrl: '', needsKey: false },
];

export function presetInfo(id: ProviderPreset): PresetInfo {
  return PRESETS.find((preset) => preset.id === id) ?? (PRESETS.at(-1) as PresetInfo);
}

export function presetLabel(id: ProviderPreset): string {
  return presetInfo(id).label ?? t('providers.preset.custom');
}

/** The entry's own name, else its preset's. */
export function providerName(entry: ProviderEntry): string {
  return entry.name.trim() || presetLabel(entry.preset);
}

export const DEFAULT_MAX_TOKENS = 4096;
export const MAX_OUTPUT_TOKENS = 1_000_000;
export const TEMPERATURE = { min: 0, max: 2, step: 0.1 } as const;

export function newProviderEntry(preset: ProviderPreset): ProviderEntry {
  const info = presetInfo(preset);
  return {
    id: `p-${crypto.randomUUID().slice(0, 8)}`,
    kind: info.kind,
    preset,
    name: '',
    baseUrl: info.baseUrl,
    defaultModel: '',
    maxTokens: DEFAULT_MAX_TOKENS,
    temperature: null,
  };
}

/** Instant feedback for the form; mirrors src-tauri/src/providers/url.rs. */
export function baseUrlProblem(raw: string): 'empty' | 'invalid' | 'insecure' | null {
  const value = raw.trim();
  if (!value) return 'empty';
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return 'invalid';
  }
  if (url.username || url.password || url.hash) return 'invalid';
  if (url.protocol === 'https:') return url.hostname ? null : 'invalid';
  if (url.protocol !== 'http:') return 'invalid';
  const host = url.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  const local = host === 'localhost' || host === '::1' || /^127\./.test(host);
  return local ? null : 'insecure';
}
