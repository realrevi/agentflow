// Applies resolved CSS variables to the document root. Only changed values are written,
// so theme switches and color-picker drags stay cheap and need no reload.
import type { ThemeKind } from '../../shared/types';
import type { CssVars } from './mapping';

let applied: CssVars = {};

export function applyThemeVars(vars: CssVars, kind: ThemeKind): void {
  const root = document.documentElement;
  for (const [name, value] of Object.entries(vars)) {
    if (applied[name] !== value) root.style.setProperty(name, value);
  }
  for (const name of Object.keys(applied)) {
    if (!(name in vars)) root.style.removeProperty(name);
  }
  applied = vars;
  root.style.colorScheme = kind;
  root.dataset.themeKind = kind;
}
