// Helpers for workspace-relative paths (always `/`-separated, '' = the root).

export function parentPath(path: string): string {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? '' : path.slice(0, slash);
}

export function baseName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

export function joinPath(dir: string, name: string): string {
  return dir ? `${dir}/${name}` : name;
}

/** True for `dir` itself and everything below it ('' contains everything). */
export function isInside(path: string, dir: string): boolean {
  return dir === '' || path === dir || path.startsWith(`${dir}/`);
}

/** Where `path` ends up after `from` was renamed to `to`; `null` if it was not affected. */
export function renamedPath(path: string, from: string, to: string): string | null {
  if (path === from) return to;
  return path.startsWith(`${from}/`) ? to + path.slice(from.length) : null;
}

/** Lowercase extension without the dot; dotfiles like `.gitignore` have none. */
export function extension(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

/** The absolute path for display/copying, using the workspace folder's own separator. */
export function absolutePath(root: string, path: string): string {
  const separator = root.includes('\\') && !root.includes('/') ? '\\' : '/';
  const base = root.replace(/[\\/]+$/, '');
  if (!path) return root;
  return `${base}${separator}${separator === '/' ? path : path.replaceAll('/', '\\')}`;
}
