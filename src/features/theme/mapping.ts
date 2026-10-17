// Maps VS Code theme `colors` to AgentFlow CSS variables (`--af-*`).
// Each variable tries its VS Code keys in order, then falls back to another variable
// (`ref`) or a per-kind default, so partial imported themes still look complete.
import { isHexColor } from '../../shared/color';
import type { ThemeKind } from '../../shared/types';
import { ANSI_SPECS } from './ansi';
import { SYNTAX_SPECS } from './syntax';

type Fallback = { ref: string } | { dark: string; light: string };

export interface VarSpec {
  /** CSS variable name without the `--af-` prefix. */
  name: string;
  keys: readonly string[];
  fallback: Fallback;
}

export type CssVars = Record<string, string>;

const d = (dark: string, light: string): Fallback => ({ dark, light });
const ref = (name: string): Fallback => ({ ref: name });

/** Order matters: a `ref` must point to a variable defined earlier in the list. */
export const THEME_VARS: readonly VarSpec[] = [
  // Base
  { name: 'bg', keys: ['editor.background'], fallback: d('#1e1e1e', '#ffffff') },
  { name: 'fg', keys: ['editor.foreground', 'foreground'], fallback: d('#d4d4d4', '#333333') },
  { name: 'fg-muted', keys: ['descriptionForeground'], fallback: d('#9a9ca3', '#6f7177') },
  {
    name: 'border',
    keys: ['panel.border', 'editorGroup.border', 'contrastBorder'],
    fallback: d('#2b2b2b', '#e1e1e1'),
  },
  {
    name: 'accent',
    keys: ['button.background', 'activityBarBadge.background', 'focusBorder'],
    fallback: d('#8b7cf6', '#6c5ce7'),
  },
  {
    name: 'accent-fg',
    keys: ['button.foreground', 'activityBarBadge.foreground'],
    fallback: d('#ffffff', '#ffffff'),
  },
  { name: 'focus', keys: ['focusBorder'], fallback: ref('accent') },
  { name: 'link', keys: ['textLink.foreground'], fallback: ref('accent') },
  { name: 'error', keys: ['errorForeground'], fallback: d('#f48771', '#a1260d') },
  { name: 'warning', keys: ['editorWarning.foreground'], fallback: d('#cca700', '#bf8803') },
  { name: 'success', keys: ['terminal.ansiGreen'], fallback: d('#89d185', '#388a34') },
  { name: 'shadow', keys: ['widget.shadow'], fallback: d('#00000066', '#00000029') },
  { name: 'selection', keys: ['editor.selectionBackground'], fallback: d('#264f78', '#add6ff') },
  // Side bar
  { name: 'sidebar-bg', keys: ['sideBar.background'], fallback: ref('bg') },
  { name: 'sidebar-fg', keys: ['sideBar.foreground'], fallback: ref('fg') },
  { name: 'sidebar-border', keys: ['sideBar.border'], fallback: ref('border') },
  { name: 'sidebar-title-fg', keys: ['sideBarTitle.foreground'], fallback: ref('sidebar-fg') },
  {
    name: 'sidebar-header-fg',
    keys: ['sideBarSectionHeader.foreground'],
    fallback: ref('fg-muted'),
  },
  // Activity bar
  { name: 'activitybar-bg', keys: ['activityBar.background'], fallback: ref('sidebar-bg') },
  { name: 'activitybar-fg', keys: ['activityBar.foreground'], fallback: ref('fg') },
  {
    name: 'activitybar-inactive-fg',
    keys: ['activityBar.inactiveForeground'],
    fallback: ref('fg-muted'),
  },
  { name: 'activitybar-border', keys: ['activityBar.border'], fallback: ref('border') },
  {
    name: 'activitybar-active-border',
    keys: ['activityBar.activeBorder'],
    fallback: ref('accent'),
  },
  { name: 'badge-bg', keys: ['activityBarBadge.background'], fallback: ref('accent') },
  { name: 'badge-fg', keys: ['activityBarBadge.foreground'], fallback: ref('accent-fg') },
  // Status bar
  { name: 'statusbar-bg', keys: ['statusBar.background'], fallback: ref('activitybar-bg') },
  { name: 'statusbar-fg', keys: ['statusBar.foreground'], fallback: ref('fg-muted') },
  { name: 'statusbar-border', keys: ['statusBar.border'], fallback: ref('border') },
  {
    name: 'statusbar-hover-bg',
    keys: ['statusBarItem.hoverBackground'],
    fallback: d('#ffffff1f', '#0000001a'),
  },
  // Tabs (dockview headers)
  { name: 'tabs-bg', keys: ['editorGroupHeader.tabsBackground'], fallback: ref('sidebar-bg') },
  { name: 'tab-active-bg', keys: ['tab.activeBackground'], fallback: ref('bg') },
  { name: 'tab-active-fg', keys: ['tab.activeForeground'], fallback: ref('fg') },
  { name: 'tab-inactive-bg', keys: ['tab.inactiveBackground'], fallback: ref('tabs-bg') },
  { name: 'tab-inactive-fg', keys: ['tab.inactiveForeground'], fallback: ref('fg-muted') },
  {
    name: 'tab-unfocused-active-fg',
    keys: ['tab.unfocusedActiveForeground'],
    fallback: ref('tab-active-fg'),
  },
  { name: 'tab-border', keys: ['tab.border'], fallback: ref('border') },
  {
    name: 'tab-active-border',
    keys: ['tab.activeBorderTop', 'tab.activeBorder'],
    fallback: ref('accent'),
  },
  { name: 'tab-hover-bg', keys: ['tab.hoverBackground'], fallback: ref('tab-inactive-bg') },
  // Panels
  { name: 'panel-bg', keys: ['panel.background'], fallback: ref('bg') },
  { name: 'panel-border', keys: ['panel.border'], fallback: ref('border') },
  { name: 'group-border', keys: ['editorGroup.border'], fallback: ref('border') },
  // Controls
  { name: 'input-bg', keys: ['input.background'], fallback: d('#2a2b33', '#ffffff') },
  { name: 'input-fg', keys: ['input.foreground'], fallback: ref('fg') },
  { name: 'input-border', keys: ['input.border'], fallback: ref('border') },
  { name: 'button-bg', keys: ['button.background'], fallback: ref('accent') },
  { name: 'button-fg', keys: ['button.foreground'], fallback: ref('accent-fg') },
  { name: 'button-hover-bg', keys: ['button.hoverBackground'], fallback: ref('button-bg') },
  {
    name: 'button-secondary-bg',
    keys: ['button.secondaryBackground'],
    fallback: d('#3a3d41', '#e4e4e7'),
  },
  { name: 'button-secondary-fg', keys: ['button.secondaryForeground'], fallback: ref('fg') },
  { name: 'list-hover-bg', keys: ['list.hoverBackground'], fallback: d('#ffffff12', '#0000000d') },
  {
    name: 'list-active-bg',
    keys: ['list.activeSelectionBackground'],
    fallback: d('#ffffff1f', '#0000001a'),
  },
  { name: 'list-active-fg', keys: ['list.activeSelectionForeground'], fallback: ref('fg') },
  {
    name: 'widget-bg',
    keys: ['editorWidget.background', 'dropdown.background', 'menu.background'],
    fallback: ref('sidebar-bg'),
  },
  { name: 'widget-fg', keys: ['editorWidget.foreground', 'menu.foreground'], fallback: ref('fg') },
  {
    name: 'widget-border',
    keys: ['editorWidget.border', 'menu.border', 'widget.border'],
    fallback: ref('border'),
  },
  {
    name: 'scrollbar',
    keys: ['scrollbarSlider.background'],
    fallback: d('#79797966', '#64646466'),
  },
  {
    name: 'scrollbar-hover',
    keys: ['scrollbarSlider.hoverBackground'],
    fallback: ref('scrollbar'),
  },
  // Terminal (used from phase 2)
  { name: 'terminal-bg', keys: ['terminal.background'], fallback: ref('panel-bg') },
  { name: 'terminal-fg', keys: ['terminal.foreground'], fallback: ref('fg') },
  { name: 'terminal-cursor', keys: ['terminalCursor.foreground'], fallback: ref('terminal-fg') },
  {
    name: 'terminal-selection',
    keys: ['terminal.selectionBackground'],
    fallback: ref('selection'),
  },
  ...ANSI_SPECS,
  // Diffs (agent permission cards): line backgrounds for added and removed text.
  {
    name: 'diff-inserted',
    keys: ['diffEditor.insertedLineBackground', 'diffEditor.insertedTextBackground'],
    fallback: d('#3fb95026', '#2ea04326'),
  },
  {
    name: 'diff-removed',
    keys: ['diffEditor.removedLineBackground', 'diffEditor.removedTextBackground'],
    fallback: d('#f8514926', '#cf222e1f'),
  },
  // Editor (phase 3). Background and text use `bg` / `fg` (editor.background/foreground).
  { name: 'editor-cursor', keys: ['editorCursor.foreground'], fallback: ref('accent') },
  {
    name: 'editor-line-highlight',
    keys: ['editor.lineHighlightBackground'],
    fallback: d('#ffffff0a', '#0000000a'),
  },
  { name: 'editor-gutter-fg', keys: ['editorLineNumber.foreground'], fallback: ref('fg-muted') },
  {
    name: 'editor-gutter-active-fg',
    keys: ['editorLineNumber.activeForeground'],
    fallback: ref('fg'),
  },
  {
    name: 'editor-selection-match',
    keys: ['editor.selectionHighlightBackground'],
    fallback: d('#ffffff1a', '#0000001a'),
  },
  {
    name: 'editor-find-match',
    keys: ['editor.findMatchHighlightBackground'],
    fallback: d('#ea5c0055', '#ea5c0040'),
  },
  { name: 'editor-bracket', keys: ['editorBracketMatch.border'], fallback: ref('fg-muted') },
  ...SYNTAX_SPECS,
];

export const cssVar = (name: string): string => `--af-${name}`;

/** Resolves every `--af-*` variable for a theme. Invalid color values are ignored. */
export function mapThemeColors(colors: Record<string, string>, kind: ThemeKind): CssVars {
  const vars: CssVars = {};
  for (const spec of THEME_VARS) {
    const fromTheme = spec.keys.map((key) => colors[key]).find((v) => v && isHexColor(v));
    vars[cssVar(spec.name)] = fromTheme ?? resolveFallback(spec.fallback, kind, vars);
  }
  return vars;
}

function resolveFallback(fallback: Fallback, kind: ThemeKind, resolved: CssVars): string {
  if ('ref' in fallback) {
    const value = resolved[cssVar(fallback.ref)];
    if (value === undefined) throw new Error(`theme var "${fallback.ref}" referenced too early`);
    return value;
  }
  return fallback[kind];
}
