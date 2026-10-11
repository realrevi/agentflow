import { MAX_RECENT_WORKSPACES } from '../settings/defaults';

/** Puts `path` first, removes duplicates and keeps the 10 most recent. */
export function addRecent(recent: readonly string[], path: string): string[] {
  return [path, ...recent.filter((entry) => entry !== path)].slice(0, MAX_RECENT_WORKSPACES);
}

/** Last path segment, for both `/` and `\\` separators ("C:\\" and "/" stay as-is). */
export function folderName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '');
  if (!trimmed) return path;
  const segment = trimmed.split(/[\\/]/).pop() ?? trimmed;
  return segment.endsWith(':') ? path : segment;
}
