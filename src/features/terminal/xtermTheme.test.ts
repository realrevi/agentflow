import { describe, expect, it } from 'vitest';
import dark from '../../../themes/agentflow-dark.json';
import { mapThemeColors } from '../theme/mapping';
import { computeThemeVars } from '../theme/overrides';
import { DEFAULT_SETTINGS } from '../settings/defaults';
import { xtermTheme } from './xtermTheme';

describe('xtermTheme', () => {
  it('maps terminal and ANSI colors from --af- variables', () => {
    const vars = mapThemeColors(dark.colors, 'dark');
    const theme = xtermTheme(vars);
    expect(theme.background).toBe(dark.colors['terminal.background']);
    expect(theme.foreground).toBe(dark.colors['terminal.foreground']);
    expect(theme.cursor).toBe(dark.colors['terminalCursor.foreground']);
    expect(theme.selectionBackground).toBe(dark.colors['terminal.selectionBackground']);
    expect(theme.red).toBe(dark.colors['terminal.ansiRed']);
    expect(theme.brightWhite).toBe(dark.colors['terminal.ansiBrightWhite']);
    expect(theme.cursorAccent).toBe(theme.background);
  });

  it('fills every ANSI slot, even for a partial theme', () => {
    const theme = xtermTheme(mapThemeColors({ 'editor.background': '#101010' }, 'light'));
    const names = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white'];
    for (const name of names) {
      const bright = `bright${name[0]?.toUpperCase()}${name.slice(1)}`;
      expect(theme[name as keyof typeof theme], name).toMatch(/^#/);
      expect(theme[bright as keyof typeof theme], bright).toMatch(/^#/);
    }
    expect(theme.background).toBe('#101010');
  });

  it('follows user overrides through the computed variables', () => {
    const vars = computeThemeVars(
      { colors: dark.colors, kind: 'dark' },
      { ...DEFAULT_SETTINGS.appearance, textColor: '#abcdef' },
    );
    expect(xtermTheme(vars).background).toBe(dark.colors['terminal.background']);
  });

  it('omits colors that are missing instead of passing undefined', () => {
    const theme = xtermTheme({ '--af-terminal-bg': '#000000' });
    expect(theme).toEqual({ background: '#000000', cursorAccent: '#000000' });
  });
});
