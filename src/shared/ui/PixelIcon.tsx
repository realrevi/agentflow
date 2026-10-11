// Renders pixel art from text rows as crisp inline SVG. Each palette character maps to
// a fill (default `#` = currentColor); `.` and spaces are transparent.
import { For, type JSX } from 'solid-js';

export interface PixelArt {
  rows: readonly string[];
  palette?: Readonly<Record<string, string>>;
}

interface PixelPath {
  fill: string;
  d: string;
}

const DEFAULT_PALETTE: Readonly<Record<string, string>> = { '#': 'currentColor' };
const pathCache = new WeakMap<PixelArt, PixelPath[]>();

/** Merges horizontal runs of same-colored pixels into one path per color. */
function toPaths(art: PixelArt): PixelPath[] {
  const cached = pathCache.get(art);
  if (cached) return cached;
  const palette = art.palette ?? DEFAULT_PALETTE;
  const byChar = new Map<string, string>();
  art.rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const char = row.charAt(x);
      let end = x + 1;
      while (row.charAt(end) === char) end++;
      if (palette[char]) {
        byChar.set(char, `${byChar.get(char) ?? ''}M${x} ${y}h${end - x}v1h-${end - x}z`);
      }
      x = end;
    }
  });
  const paths = [...byChar].map(([char, d]) => ({ fill: palette[char] ?? 'currentColor', d }));
  pathCache.set(art, paths);
  return paths;
}

export function PixelIcon(props: { art: PixelArt; scale?: number; class?: string }): JSX.Element {
  const width = () => props.art.rows[0]?.length ?? 0;
  const height = () => props.art.rows.length;
  const scale = () => props.scale ?? 2;
  return (
    <svg
      class={props.class}
      width={width() * scale()}
      height={height() * scale()}
      viewBox={`0 0 ${width()} ${height()}`}
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <For each={toPaths(props.art)}>{(path) => <path d={path.d} fill={path.fill} />}</For>
    </svg>
  );
}
