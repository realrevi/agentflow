// 8x8 pixel icons for tool cards, permission cards and the mode control (16px at 2x).
import type { PixelArt } from '../../../shared/ui/PixelIcon';

const folder: PixelArt = {
  rows: [
    '........',
    '.###....',
    '.#..####',
    '.#.....#',
    '.#.....#',
    '.#.....#',
    '.#######',
    '........',
  ],
};

const file: PixelArt = {
  rows: [
    '.####...',
    '.#..##..',
    '.#...##.',
    '.#.##.#.',
    '.#....#.',
    '.#.##.#.',
    '.#....#.',
    '.######.',
  ],
};

const magnifier: PixelArt = {
  rows: [
    '.####...',
    '#....#..',
    '#....#..',
    '#....#..',
    '.####...',
    '....##..',
    '.....##.',
    '......##',
  ],
};

const stars: PixelArt = {
  rows: [
    '.#......',
    '###...#.',
    '.#...###',
    '......#.',
    '..#.....',
    '.###....',
    '..#..#..',
    '....###.',
  ],
};

const pencil: PixelArt = {
  rows: [
    '.....##.',
    '....#..#',
    '...#..#.',
    '..#..#..',
    '.#..#...',
    '#..#....',
    '##.#....',
    '###.....',
  ],
};

const plusFile: PixelArt = {
  rows: [
    '.####...',
    '.#..##..',
    '.#...##.',
    '.#..#.#.',
    '.#.###..',
    '.#..#.#.',
    '.#....#.',
    '.######.',
  ],
};

const prompt: PixelArt = {
  rows: [
    '########',
    '#......#',
    '#.#....#',
    '#..#...#',
    '#.#.##.#',
    '#......#',
    '########',
    '........',
  ],
};

export const thinkIcon: PixelArt = {
  rows: [
    '..####..',
    '.#....#.',
    '#..##..#',
    '#....#.#',
    '.#..#..#',
    '..#....#',
    '...####.',
    '..#.....',
  ],
};

export const shieldIcon: PixelArt = {
  rows: [
    '.######.',
    '#......#',
    '#..##..#',
    '#..##..#',
    '#......#',
    '.#.##.#.',
    '..#..#..',
    '...##...',
  ],
};

export const eyeIcon: PixelArt = {
  rows: [
    '........',
    '..####..',
    '.#....#.',
    '#..##..#',
    '#..##..#',
    '.#....#.',
    '..####..',
    '........',
  ],
};

export const undoIcon: PixelArt = {
  rows: [
    '..#.....',
    '.##.....',
    '#######.',
    '.##....#',
    '..#....#',
    '.......#',
    '..#####.',
    '........',
  ],
};

const ICONS: Record<string, PixelArt> = {
  list_dir: folder,
  read_file: file,
  glob: stars,
  search: magnifier,
  write_file: plusFile,
  edit_file: pencil,
  run_command: prompt,
};

export function toolIcon(name: string): PixelArt {
  return ICONS[name] ?? prompt;
}

export { folder as exploreIcon, pencil as editIcon };
