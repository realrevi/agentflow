// The AgentFlow logo as pixel art (12x12 at 2x): the one pixel accent among the UI icons,
// which are Lucide (Icon.tsx).
import type { PixelArt } from './PixelIcon';

export const logoIcon: PixelArt = {
  rows: [
    '.....oo.....',
    '......#.....',
    '..########..',
    '.##########.',
    '.##oo##oo##.',
    '.##oo##oo##.',
    '.##########.',
    '.###....###.',
    '.##########.',
    '..#......#..',
    '............',
    '............',
  ],
  palette: { '#': 'var(--af-accent)', o: 'var(--af-bold-color)' },
};
