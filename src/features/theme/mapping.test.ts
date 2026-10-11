import { describe, expect, it } from 'vitest';
import dark from '../../../themes/agentflow-dark.json';
import light from '../../../themes/agentflow-light.json';
import crt from '../../../themes/retro-crt.json';
import { isHexColor } from '../../shared/color';
import type { ThemeKind } from '../../shared/types';
import { cssVar, mapThemeColors, THEME_VARS } from './mapping';

const isCssColor = (value: string) => isHexColor(value) || value === 'transparent';

describe('mapThemeColors', () => {
  it('defines every variable with a valid color, even for an empty theme', () => {
    for (const kind of ['dark', 'light'] as ThemeKind[]) {
      const vars = mapThemeColors({}, kind);
      expect(Object.keys(vars)).toHaveLength(THEME_VARS.length);
      for (const [name, value] of Object.entries(vars)) {
        expect(name.startsWith('--af-'), name).toBe(true);
        expect(isCssColor(value), `${name}=${value}`).toBe(true);
      }
    }
  });

  it('uses per-kind defaults when a key is missing', () => {
    expect(mapThemeColors({}, 'dark')['--af-bg']).toBe('#1e1e1e');
    expect(mapThemeColors({}, 'light')['--af-bg']).toBe('#ffffff');
  });

  it('maps VS Code keys to --af- variables', () => {
    const vars = mapThemeColors(
      {
        'editor.background': '#101010',
        'sideBar.background': '#202020',
        'activityBar.background': '#303030',
        'statusBar.background': '#404040',
        'tab.activeBackground': '#505050',
        'panel.background': '#606060',
        'terminal.ansiBrightRed': '#ff0000',
      },
      'dark',
    );
    expect(vars['--af-bg']).toBe('#101010');
    expect(vars['--af-sidebar-bg']).toBe('#202020');
    expect(vars['--af-activitybar-bg']).toBe('#303030');
    expect(vars['--af-statusbar-bg']).toBe('#404040');
    expect(vars['--af-tab-active-bg']).toBe('#505050');
    expect(vars['--af-panel-bg']).toBe('#606060');
    expect(vars['--af-ansi-bright-red']).toBe('#ff0000');
  });

  it('follows fallback chains for partial themes', () => {
    const vars = mapThemeColors(
      { 'editor.background': '#123456', 'button.background': '#abcdef' },
      'dark',
    );
    expect(vars['--af-sidebar-bg']).toBe('#123456');
    expect(vars['--af-activitybar-bg']).toBe('#123456');
    expect(vars['--af-terminal-bg']).toBe('#123456');
    expect(vars['--af-accent']).toBe('#abcdef');
    expect(vars['--af-tab-active-border']).toBe('#abcdef');
  });

  it('prefers earlier keys and ignores invalid values', () => {
    const vars = mapThemeColors(
      {
        'editor.foreground': 'red',
        foreground: '#eeeeee',
        'tab.activeBorderTop': '#111',
        'tab.activeBorder': '#222',
      },
      'dark',
    );
    expect(vars['--af-fg']).toBe('#eeeeee');
    expect(vars['--af-tab-active-border']).toBe('#111');
  });

  it('only references variables that are already resolved', () => {
    const seen = new Set<string>();
    for (const spec of THEME_VARS) {
      if ('ref' in spec.fallback) expect(seen.has(spec.fallback.ref), spec.name).toBe(true);
      seen.add(spec.name);
    }
  });

  it('builds all 16 ANSI colors', () => {
    const ansi = THEME_VARS.filter((spec) => spec.name.startsWith('ansi-')).map(
      (spec) => spec.name,
    );
    expect(ansi).toHaveLength(16);
    expect(ansi).toContain('ansi-black');
    expect(ansi).toContain('ansi-bright-white');
  });

  it('built-in themes set the core UI colors themselves', () => {
    for (const theme of [dark, light, crt]) {
      const kind = theme.type as ThemeKind;
      const vars = mapThemeColors(theme.colors, kind);
      const core = ['bg', 'fg', 'sidebar-bg', 'activitybar-bg', 'statusbar-bg', 'tab-active-bg'];
      for (const name of core) {
        expect(Object.values(theme.colors), `${theme.name}: ${name}`).toContain(vars[cssVar(name)]);
      }
    }
  });
});
