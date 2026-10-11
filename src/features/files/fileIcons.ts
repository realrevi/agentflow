// 8x8 pixel-art file icons (rendered at 16px). Code languages are letter badges, data and
// markup formats are glyphs. Colors come from theme variables (the terminal ANSI palette).
import type { PixelArt } from '../../shared/ui/PixelIcon';
import { extension } from '../workspace';

const color = (name: string) => `var(--af-${name})`;
const glyph = (rows: string[], main: string, second?: string): PixelArt => ({
  rows,
  palette: second ? { '#': color(main), o: color(second) } : { '#': color(main) },
});

const PAGE = ['.####...', '.#..##..', '.#...##.'];

export const folderIcon = glyph(
  ['........', '###.....', '########', '#......#', '#......#', '#......#', '########', '........'],
  'accent',
);
export const folderOpenIcon = glyph(
  ['........', '###.....', '#..####.', '#.######', '##.....#', '#.....#.', '######..', '........'],
  'accent',
);
const fileIcon = glyph(
  [...PAGE, '.#....#.', '.#....#.', '.#....#.', '.######.', '........'],
  'fg-muted',
);
const textIcon = glyph(
  [...PAGE, '.#oooo#.', '.#....#.', '.#ooo.#.', '.######.', '........'],
  'fg-muted',
  'fg',
);
const imageIcon = glyph(
  ['########', '#......#', '#.o....#', '#....#.#', '#...####', '#.######', '########', '........'],
  'ansi-green',
  'ansi-yellow',
);
const jsonIcon = glyph(
  ['.##..##.', '.#....#.', '.#....#.', '#......#', '.#....#.', '.#....#.', '.##..##.', '........'],
  'ansi-yellow',
);
const htmlIcon = glyph(
  ['........', '..#..#..', '.#....#.', '#......#', '.#....#.', '..#..#..', '........', '........'],
  'ansi-red',
);
const cssIcon = glyph(
  ['..#..#..', '..#..#..', '########', '..#..#..', '########', '..#..#..', '..#..#..', '........'],
  'ansi-bright-blue',
);
const configIcon = glyph(
  ['........', '#.######', '........', '#.####..', '........', '#.#####.', '........', '........'],
  'ansi-magenta',
);

/** A rounded badge with the letter cut out of it (rows 1-6 are the inside). */
const badge = (inside: string[], main: string): PixelArt =>
  glyph(['.######.', ...inside, '.######.'], main);

const tsIcon = badge(
  ['#......#', '###..###', '###..###', '###..###', '###..###', '########'],
  'ansi-blue',
);
const jsIcon = badge(
  ['#####..#', '#####..#', '#####..#', '#..##..#', '##....##', '########'],
  'ansi-yellow',
);
const pyIcon: PixelArt = {
  rows: [
    '.######.',
    '#.....##',
    '#.###..#',
    '#.....##',
    '#.oooooo',
    '#.oooooo',
    'oooooooo',
    '.oooooo.',
  ],
  palette: { '#': color('ansi-blue'), o: color('ansi-yellow') },
};
const rsIcon = badge(
  ['#....###', '#.##..##', '#....###', '#.##.###', '#.###..#', '########'],
  'ansi-red',
);
const cIcon = badge(
  ['##....##', '#..#####', '#.######', '#..#####', '##....##', '########'],
  'ansi-blue',
);
const cppIcon = badge(
  ['###..###', '###..###', '#......#', '#......#', '###..###', '###..###'],
  'ansi-cyan',
);
const headerIcon = badge(
  ['#.####.#', '#.####.#', '#......#', '#.####.#', '#.####.#', '########'],
  'ansi-magenta',
);
const csIcon = badge(
  ['##.##.##', '#......#', '##.##.##', '#......#', '##.##.##', '########'],
  'ansi-green',
);
const mdIcon = badge(
  ['#.####.#', '#..##..#', '#.#..#.#', '#.####.#', '#.####.#', '########'],
  'ansi-bright-cyan',
);

const BY_EXTENSION: Record<string, PixelArt> = {
  md: mdIcon,
  markdown: mdIcon,
  json: jsonIcon,
  jsonc: jsonIcon,
  txt: textIcon,
  log: textIcon,
  py: pyIcon,
  cs: csIcon,
  c: cIcon,
  cpp: cppIcon,
  cc: cppIcon,
  cxx: cppIcon,
  h: headerIcon,
  hpp: headerIcon,
  hh: headerIcon,
  hxx: headerIcon,
  rs: rsIcon,
  ts: tsIcon,
  tsx: tsIcon,
  mts: tsIcon,
  cts: tsIcon,
  js: jsIcon,
  jsx: jsIcon,
  mjs: jsIcon,
  cjs: jsIcon,
  html: htmlIcon,
  htm: htmlIcon,
  css: cssIcon,
  scss: cssIcon,
  less: cssIcon,
  yaml: configIcon,
  yml: configIcon,
  toml: configIcon,
  png: imageIcon,
  jpg: imageIcon,
  jpeg: imageIcon,
  gif: imageIcon,
  svg: imageIcon,
  webp: imageIcon,
  ico: imageIcon,
  bmp: imageIcon,
};

export function iconFor(name: string, isDir: boolean, expanded = false): PixelArt {
  if (isDir) return expanded ? folderOpenIcon : folderIcon;
  return BY_EXTENSION[extension(name)] ?? fileIcon;
}

/** Tree twisty, rotated with CSS when expanded. */
export const chevronIcon = glyph(
  ['........', '..#.....', '..##....', '..###...', '..##....', '..#.....', '........', '........'],
  'fg-muted',
);
