// User overrides layered on top of any theme: base text color, accent and "agent bold".
import { isHexColor, readableOn, withAlpha } from '../../shared/color';
import type { AppearanceSettings, ThemeKind } from '../../shared/types';
import { cssVar, mapThemeColors, type CssVars } from './mapping';

export const DEFAULT_BOLD_COLOR = '#F5C542';

const TEXT_VARS = ['fg', 'sidebar-fg', 'tab-active-fg', 'input-fg', 'widget-fg'];
const ACCENT_VARS = [
  'accent',
  'focus',
  'button-bg',
  'button-hover-bg',
  'badge-bg',
  'activitybar-active-border',
  'tab-active-border',
];
const ACCENT_FG_VARS = ['accent-fg', 'button-fg', 'badge-fg'];

/**
 * Soft two-layer glow used by `.af-agent-bold`. Light themes get a thin dark outline
 * first, so bright bold colors (like the default gold) stay readable on light backgrounds.
 */
export function glowShadow(color: string, kind: ThemeKind = 'dark'): string {
  const inner = withAlpha(color, 0.75);
  const outer = withAlpha(color, 0.35);
  if (!inner || !outer) return 'none';
  const glow = `0 0 4px ${inner}, 0 0 12px ${outer}`;
  return kind === 'light' ? `0 0 1px rgba(0, 0, 0, 0.7), ${glow}` : glow;
}

export function overrideVars(appearance: AppearanceSettings, kind: ThemeKind = 'dark'): CssVars {
  const vars: CssVars = {};
  const set = (names: readonly string[], value: string) => {
    for (const name of names) vars[cssVar(name)] = value;
  };

  const { textColor, accentColor, agentBold } = appearance;
  if (textColor && isHexColor(textColor)) set(TEXT_VARS, textColor);
  if (accentColor && isHexColor(accentColor)) {
    set(ACCENT_VARS, accentColor);
    set(ACCENT_FG_VARS, readableOn(accentColor));
  }

  const bold = isHexColor(agentBold.color) ? agentBold.color : DEFAULT_BOLD_COLOR;
  vars[cssVar('bold-color')] = bold;
  vars[cssVar('glow')] = agentBold.glow ? glowShadow(bold, kind) : 'none';
  return vars;
}

/** Variables computed from other variables after overrides are applied. */
function derivedVars(vars: CssVars): CssVars {
  const accent = vars[cssVar('accent')] ?? DEFAULT_BOLD_COLOR;
  return {
    [cssVar('accent-soft')]: withAlpha(accent, 0.22) ?? 'transparent',
  };
}

export interface ThemeInput {
  colors: Record<string, string>;
  kind: ThemeKind;
}

/** Everything the document root needs for the given theme and user overrides. */
export function computeThemeVars(theme: ThemeInput, appearance: AppearanceSettings): CssVars {
  const vars = {
    ...mapThemeColors(theme.colors, theme.kind),
    ...overrideVars(appearance, theme.kind),
  };
  return { ...vars, ...derivedVars(vars) };
}
