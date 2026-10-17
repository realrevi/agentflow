// Token & cost tracking (phase 6). Mirrors src-tauri/src/{tokens,settings/usage.rs,
// chat/compact.rs,chat/estimate.rs}. Money is USD; prices are USD per 1M tokens.
import type { Attachment, Compaction, TokenUsage } from './chatTypes';

/** Where usage came from. CLI usage uses the source as its provider key. */
export type UsageSource = 'api' | 'claude-code' | 'codex' | 'gemini-cli';

export interface PriceOverride {
  model: string;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

/** `provider`: a provider entry id or a CLI key (`claude-code`, `codex`, `gemini-cli`). */
export interface ProviderBudget {
  provider: string;
  daily: number | null;
  monthly: number | null;
}

export interface Budgets {
  /** USD per local day, all sources. */
  daily: number | null;
  /** USD per calendar month, all sources. */
  monthly: number | null;
  providers: ProviderBudget[];
  /** Block API chat sends while over budget (CLI agents can only be warned). */
  hardStop: boolean;
}

export interface UsageSettings {
  /** Read Claude Code / Codex / Gemini CLI session logs (read-only). */
  trackCli: boolean;
  /** CLI cost is labelled "API-equivalent" (a subscription pays for it). */
  cliSubscription: boolean;
  /** Provider entry ids whose usage costs $0. */
  freeProviders: string[];
  prices: PriceOverride[];
  budgets: Budgets;
}

export interface Totals extends TokenUsage {
  requests: number;
  /** USD of the priced part. */
  cost: number;
  /** Tokens without a known price (their cost is shown as "—"). */
  unpricedTokens: number;
  /** What cache reads saved vs. the full input price. */
  cacheSavings: number;
  /** USD of CLI usage (labelled API-equivalent with a subscription). */
  cliCost: number;
}

export interface UsageDay extends Totals {
  /** `YYYY-MM-DD`, local. */
  date: string;
}

export interface UsageGroup extends Totals {
  /** Model id, workspace path (empty = none) or panel id. */
  key: string;
  source: UsageSource;
  /** Provider entry id or CLI key. */
  provider: string;
}

export interface UsageSummary {
  today: Totals;
  week: Totals;
  month: Totals;
  /** The last 30 local days, oldest first. */
  days: UsageDay[];
  /** Last 30 days, most expensive first (at most 50 each). */
  models: UsageGroup[];
  workspaces: UsageGroup[];
  panels: UsageGroup[];
  /** Summaries written by Compact (30 days). */
  compact: Totals;
}

export type BudgetPeriod = 'daily' | 'monthly';

export interface BudgetStatus {
  /** `all`, a provider entry id or a CLI key. */
  scope: string;
  period: BudgetPeriod;
  limit: number;
  spent: number;
  level: 0 | 80 | 100;
}

export interface AllowOnceGrant {
  /** Pass as `allowOnce` to the next `chat_send` for this provider. Single use. */
  token: string;
  providerId: string;
  expiresAt: number;
}

export interface PriceEntry {
  model: string;
  aliases: string[];
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  /** Where the price comes from (official pricing page). */
  source: string;
  /** `YYYY-MM-DD` */
  updated: string;
  note: string | null;
}

export interface DynamicPrice {
  model: string;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

export interface PriceList {
  shipped: PriceEntry[];
  /** OpenRouter prices from its models endpoint (once its model list was loaded). */
  dynamic: DynamicPrice[];
  overrides: PriceOverride[];
  freeProviders: string[];
}

export interface CliSupport {
  source: Exclude<UsageSource, 'api'>;
  /** A reliable usage source exists for this CLI. */
  available: boolean;
  root: string | null;
  /** The folder exists (the CLI has been used here). */
  found: boolean;
  watching: boolean;
}

export type WindowSource = 'endpoint' | 'setting' | 'default';

export interface Estimate {
  tokens: number;
  /** Not the provider's own tokenizer: show "≈". */
  approximate: boolean;
  /** Input cost; `null` = no known price ("—"). */
  cost: number | null;
  contextWindow: number;
  windowSource: WindowSource;
  /** tokens / contextWindow */
  share: number;
  threshold: number;
  /** Auto-compact is on and the threshold is reached. */
  shouldCompact: boolean;
}

export interface EstimateRequest {
  workspace: string | null;
  conversationId: string;
  text: string;
  attachments: Attachment[];
}

export interface CompactRequest {
  workspace: string | null;
  conversationId: string;
  panel: string | null;
}

export interface CompactResult {
  compaction: Compaction;
  /** Messages the summary replaces in requests (the chat still shows them). */
  summarized: number;
  usage: TokenUsage;
}

/** Live usage changes (Channel of `usage_subscribe`). */
export type UsageEvent =
  | { type: 'changed'; panels: string[] }
  | { type: 'repriced' }
  | { type: 'budget'; status: BudgetStatus };
