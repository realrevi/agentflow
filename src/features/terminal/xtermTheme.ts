// xterm.js colors from the active AgentFlow theme (the resolved --af-* variables).
import type { ITheme } from '@xterm/xterm';

const ANSI = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white'] as const;
type Ansi = (typeof ANSI)[number];
type BrightAnsi = `bright${Capitalize<Ansi>}`;

const capitalize = <T extends string>(word: T) =>
  (word.charAt(0).toUpperCase() + word.slice(1)) as Capitalize<T>;

export function xtermTheme(vars: Readonly<Record<string, string>>): ITheme {
  const v = (name: string): string | undefined => vars[`--af-${name}`];
  const theme: ITheme = {
    background: v('terminal-bg'),
    foreground: v('terminal-fg'),
    cursor: v('terminal-cursor'),
    cursorAccent: v('terminal-bg'),
    selectionBackground: v('terminal-selection'),
    scrollbarSliderBackground: v('scrollbar'),
    scrollbarSliderHoverBackground: v('scrollbar-hover'),
    scrollbarSliderActiveBackground: v('scrollbar-hover'),
  };
  for (const color of ANSI) {
    theme[color] = v(`ansi-${color}`);
    theme[`bright${capitalize(color)}` satisfies BrightAnsi] = v(`ansi-bright-${color}`);
  }
  // Leave unknown colors to xterm's defaults instead of passing `undefined`.
  return Object.fromEntries(
    Object.entries(theme).filter(([, value]) => typeof value === 'string'),
  ) as ITheme;
}
