import type { Settings } from '../../shared/types';

/** Used until the backend answers (and if it cannot). Mirrors `Settings::default()` in Rust. */
export const DEFAULT_SETTINGS: Settings = {
  version: 4,
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
  layout: null,
};

export const MAX_RECENT_WORKSPACES = 10;
export const TERMINAL_FONT_SIZE = { min: 8, max: 32 } as const;
export const TERMINAL_SCROLLBACK_MAX = 100_000;
export const AVATAR_SCALES = [2, 3, 4] as const;
export const ANIMATION_SPEED = { min: 0.5, max: 2, step: 0.25 } as const;

export const SIDEBAR_MIN_WIDTH = 180;
export const SIDEBAR_MAX_WIDTH = 600;
