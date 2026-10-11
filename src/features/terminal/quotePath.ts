/**
 * Quotes a workspace-relative path for typing into a shell: single quotes for POSIX
 * shells, double quotes and backslashes on Windows (cmd and PowerShell both accept it).
 */
export function quotePath(path: string, windows: boolean): string {
  if (windows) return `"${path.replaceAll('/', '\\')}"`;
  return `'${path.replaceAll("'", `'\\''`)}'`;
}
