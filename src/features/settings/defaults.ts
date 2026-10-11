import type { Settings } from '../../shared/types';

/** Used until the backend answers (and if it cannot). Mirrors `Settings::default()` in Rust. */
export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  language: 'en',
  appearance: {
    themeId: 'agentflow-dark',
    textColor: null,
    accentColor: null,
    agentBold: { color: '#F5C542', glow: true },
  },
  ui: { sidebarVisible: true, sidebarWidth: 260, activeView: 'agents' },
  layout: null,
};

export const SIDEBAR_MIN_WIDTH = 180;
export const SIDEBAR_MAX_WIDTH = 600;
