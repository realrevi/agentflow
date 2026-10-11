// Syntax highlighting colors for the editor: `--af-syntax-*`. A VS Code theme's
// `tokenColors` arrive as `agentflow.syntax.*` keys (derived by the backend); themes
// without them fall back to their terminal ANSI palette, so every theme looks coherent.
import type { VarSpec } from './mapping';

const SYNTAX: readonly [name: string, fallback: string][] = [
  ['keyword', 'ansi-magenta'],
  ['string', 'ansi-green'],
  ['number', 'ansi-yellow'],
  ['constant', 'ansi-yellow'],
  ['comment', 'fg-muted'],
  ['function', 'ansi-bright-blue'],
  ['type', 'ansi-bright-cyan'],
  ['property', 'ansi-cyan'],
  ['tag', 'ansi-red'],
  ['attribute', 'ansi-yellow'],
  ['variable', 'fg'],
  ['meta', 'ansi-bright-magenta'],
  ['heading', 'accent'],
];

export const SYNTAX_CATEGORIES = SYNTAX.map(([name]) => name);

export const SYNTAX_SPECS: readonly VarSpec[] = SYNTAX.map(([name, fallback]) => ({
  name: `syntax-${name}`,
  keys: [`agentflow.syntax.${name}`],
  fallback: { ref: fallback },
}));
