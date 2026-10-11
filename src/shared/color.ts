// Tiny hex color helpers (no dependency). Accepts #rgb, #rgba, #rrggbb, #rrggbbaa.

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export function isHexColor(value: string): boolean {
  return HEX.test(value);
}

export function parseHex(value: string): Rgba | null {
  if (!HEX.test(value)) return null;
  let digits = value.slice(1);
  if (digits.length <= 4) digits = [...digits].map((d) => d + d).join('');
  const byte = (i: number) => parseInt(digits.slice(i, i + 2), 16);
  return { r: byte(0), g: byte(2), b: byte(4), a: digits.length === 8 ? byte(6) / 255 : 1 };
}

/** `#rrggbb` (alpha dropped), the only format `<input type="color">` accepts. */
export function toHex6(value: string): string | null {
  const c = parseHex(value);
  if (!c) return null;
  return '#' + [c.r, c.g, c.b].map((n) => n.toString(16).padStart(2, '0')).join('');
}

export function withAlpha(value: string, alpha: number): string | null {
  const c = parseHex(value);
  return c ? `rgba(${c.r}, ${c.g}, ${c.b}, ${alpha})` : null;
}

/** Perceived brightness in 0..1 (ignores alpha). */
export function luminance(value: string): number {
  const c = parseHex(value);
  if (!c) return 0;
  return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) / 255;
}

/** Black or white, whichever reads better on `background`. */
export function readableOn(background: string): string {
  return luminance(background) > 0.6 ? '#000000' : '#ffffff';
}
