import { describe, expect, it } from 'vitest';
import type { AppearanceSettings } from '../../shared/types';
import { computeThemeVars, DEFAULT_BOLD_COLOR, glowShadow, overrideVars } from './overrides';

const base: AppearanceSettings = {
  themeId: 'agentflow-dark',
  textColor: null,
  accentColor: null,
  agentBold: { color: DEFAULT_BOLD_COLOR, glow: true },
};

describe('overrideVars', () => {
  it('only sets agent bold variables when nothing is overridden', () => {
    expect(Object.keys(overrideVars(base)).sort()).toEqual(['--af-bold-color', '--af-glow']);
  });

  it('applies the base text color', () => {
    const vars = overrideVars({ ...base, textColor: '#abcdef' });
    expect(vars['--af-fg']).toBe('#abcdef');
    expect(vars['--af-sidebar-fg']).toBe('#abcdef');
  });

  it('applies the accent with a readable foreground', () => {
    const lightAccent = overrideVars({ ...base, accentColor: '#ffee00' });
    expect(lightAccent['--af-accent']).toBe('#ffee00');
    expect(lightAccent['--af-button-bg']).toBe('#ffee00');
    expect(lightAccent['--af-accent-fg']).toBe('#000000');
    expect(overrideVars({ ...base, accentColor: '#202060' })['--af-accent-fg']).toBe('#ffffff');
  });

  it('turns the glow on and off', () => {
    const on = overrideVars({ ...base, agentBold: { color: '#ff0000', glow: true } });
    expect(on['--af-bold-color']).toBe('#ff0000');
    expect(on['--af-glow']).toBe('0 0 4px rgba(255, 0, 0, 0.75), 0 0 12px rgba(255, 0, 0, 0.35)');
    const off = overrideVars({ ...base, agentBold: { color: '#ff0000', glow: false } });
    expect(off['--af-glow']).toBe('none');
  });

  it('adds a dark outline to the glow on light themes', () => {
    const light = overrideVars({ ...base, agentBold: { color: '#ff0000', glow: true } }, 'light');
    expect(light['--af-glow']).toMatch(
      /^0 0 1px rgba\(0, 0, 0, 0.7\), 0 0 4px rgba\(255, 0, 0, 0.75\)/,
    );
  });

  it('ignores invalid colors', () => {
    const vars = overrideVars({
      ...base,
      textColor: 'blue',
      agentBold: { color: 'nope', glow: false },
    });
    expect(vars['--af-fg']).toBeUndefined();
    expect(vars['--af-bold-color']).toBe(DEFAULT_BOLD_COLOR);
    expect(glowShadow('nope')).toBe('none');
  });
});

describe('computeThemeVars', () => {
  it('layers overrides over the theme and derives the soft accent', () => {
    const theme = {
      colors: { 'editor.foreground': '#111111', 'button.background': '#0000ff' },
      kind: 'dark' as const,
    };
    const plain = computeThemeVars(theme, base);
    expect(plain['--af-fg']).toBe('#111111');
    expect(plain['--af-accent-soft']).toBe('rgba(0, 0, 255, 0.22)');

    const custom = computeThemeVars(theme, {
      ...base,
      textColor: '#eeeeee',
      accentColor: '#ff0000',
    });
    expect(custom['--af-fg']).toBe('#eeeeee');
    expect(custom['--af-accent-soft']).toBe('rgba(255, 0, 0, 0.22)');
  });
});
