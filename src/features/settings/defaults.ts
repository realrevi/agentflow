import type { Settings } from '../../shared/types';

/** Used until the backend answers (and if it cannot). Mirrors `Settings::default()` in Rust. */
export const DEFAULT_SETTINGS: Settings = {
  version: 8,
  language: 'en',
  appearance: {
    themeId: 'agentflow-dark',
    textColor: null,
    accentColor: null,
    agentBold: { color: '#F5C542', glow: true },
  },
  ui: { sidebarVisible: true, sidebarWidth: 260, activeView: 'agents' },
  currentWorkspace: null,
  recentWorkspaces: [],
  terminal: { fontSize: 13, scrollback: 5000, cursorStyle: 'block', autoStartShells: true },
  files: { showHidden: false },
  characters: {
    enabled: true,
    cursorFollow: true,
    packet: true,
    bubble: true,
    typewriter: true,
    speed: 1,
    scale: 3,
    onShells: false,
  },
  providers: [],
  chat: {
    smoothReveal: true,
    promptCaching: true,
    defaultContextWindow: 128_000,
    contextWindows: [],
    compactThreshold: 0.7,
    compactKeepTurns: 4,
    compactProviderId: null,
    compactModel: '',
    autoCompact: false,
  },
  usage: {
    trackCli: true,
    cliSubscription: false,
    freeProviders: [],
    prices: [],
    budgets: { daily: null, monthly: null, providers: [], hardStop: false },
  },
  agent: {
    maxIterations: 25,
    toolResultCap: 20_000,
    commandTimeoutSecs: 120,
    defaultMode: 'ask',
    defaultPrompt: true,
    showReasoning: true,
    toolModels: [],
  },
  workstreams: {
    glow: true,
    glowColor: null,
    notify: false,
    defaultMode: 'semiAuto',
    maxRounds: 10,
    reportCapKb: 30,
    stageScale: 4,
    stageAnimations: true,
  },
  layout: null,
};

export const MAX_RECENT_WORKSPACES = 10;
export const TERMINAL_FONT_SIZE = { min: 8, max: 32 } as const;
export const TERMINAL_SCROLLBACK_MAX = 100_000;
export const AVATAR_SCALES = [2, 3, 4] as const;
export const STAGE_SCALES = [2, 3, 4, 5, 6] as const;
export const MAX_ROUNDS = { min: 1, max: 100 } as const;
export const REPORT_CAP_KB = { min: 1, max: 1024 } as const;
export const ANIMATION_SPEED = { min: 0.5, max: 2, step: 0.25 } as const;

export const SIDEBAR_MIN_WIDTH = 180;
export const SIDEBAR_MAX_WIDTH = 600;
