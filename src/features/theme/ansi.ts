// The 16 terminal ANSI colors: `terminal.ansiBrightRed` -> `--af-ansi-bright-red`.
// Defaults are VS Code's built-in terminal palettes.
import type { VarSpec } from './mapping';

const NAMES = ['Black', 'Red', 'Green', 'Yellow', 'Blue', 'Magenta', 'Cyan', 'White'] as const;

// prettier-ignore
const DARK = [
  '#000000', '#cd3131', '#0dbc79', '#e5e510', '#2472c8', '#bc3fbc', '#11a8cd', '#e5e5e5',
  '#666666', '#f14c4c', '#23d18b', '#f5f543', '#3b8eea', '#d670d6', '#29b8db', '#e5e5e5',
];
// prettier-ignore
const LIGHT = [
  '#000000', '#cd3131', '#00bc00', '#949800', '#0451a5', '#bc05bc', '#0598bc', '#555555',
  '#666666', '#cd3131', '#14ce14', '#b5ba00', '#0451a5', '#bc05bc', '#0598bc', '#a5a5a5',
];

export const ANSI_SPECS: readonly VarSpec[] = [false, true].flatMap((bright) =>
  NAMES.map((color, i): VarSpec => {
    const index = bright ? i + 8 : i;
    return {
      name: `ansi-${bright ? 'bright-' : ''}${color.toLowerCase()}`,
      keys: [`terminal.ansi${bright ? 'Bright' : ''}${color}`],
      fallback: { dark: DARK[index] ?? '#000000', light: LIGHT[index] ?? '#000000' },
    };
  }),
);
