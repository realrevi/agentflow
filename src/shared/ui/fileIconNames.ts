// File and folder icons: a subset of Material Icon Theme (MIT, see
// file-icons/LICENSE-material-icon-theme.txt). Special file names first, then the
// extension, then a generic file; a few well-known folders get their own icon.

const BY_NAME: Record<string, string> = {
  'package.json': 'nodejs',
  'package-lock.json': 'npm',
  '.npmrc': 'npm',
  'pnpm-lock.yaml': 'pnpm',
  'pnpm-workspace.yaml': 'pnpm',
  'yarn.lock': 'yarn',
  '.yarnrc': 'yarn',
  'cargo.toml': 'rust',
  'cargo.lock': 'lock',
  dockerfile: 'docker',
  'docker-compose.yml': 'docker',
  'docker-compose.yaml': 'docker',
  '.dockerignore': 'docker',
  '.gitignore': 'git',
  '.gitattributes': 'git',
  '.gitmodules': 'git',
  license: 'license',
  'license.md': 'license',
  'license.txt': 'license',
  'readme.md': 'readme',
  readme: 'readme',
  makefile: 'makefile',
  'tsconfig.json': 'tsconfig',
  'jsconfig.json': 'tsconfig',
  'vite.config.ts': 'vite',
  'vite.config.js': 'vite',
  '.env': 'tune',
  '.editorconfig': 'settings',
};

const BY_EXTENSION: Record<string, string> = {
  ts: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  tsx: 'react_ts',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'react',
  json: 'json',
  jsonc: 'json',
  md: 'markdown',
  markdown: 'markdown',
  mdc: 'markdown',
  rs: 'rust',
  py: 'python',
  html: 'html',
  htm: 'html',
  css: 'css',
  scss: 'sass',
  sass: 'sass',
  less: 'less',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  ico: 'image',
  bmp: 'image',
  svg: 'svg',
  go: 'go',
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  c: 'c',
  cc: 'cpp',
  cpp: 'cpp',
  cxx: 'cpp',
  h: 'h',
  hpp: 'hpp',
  hh: 'hpp',
  hxx: 'hpp',
  cs: 'csharp',
  php: 'php',
  rb: 'ruby',
  swift: 'swift',
  lua: 'lua',
  sh: 'console',
  bash: 'console',
  zsh: 'console',
  fish: 'console',
  bat: 'console',
  cmd: 'console',
  ps1: 'powershell',
  psm1: 'powershell',
  lock: 'lock',
  xml: 'xml',
  sql: 'database',
  db: 'database',
  sqlite: 'database',
  pdf: 'pdf',
  zip: 'zip',
  gz: 'zip',
  tgz: 'zip',
  '7z': 'zip',
  rar: 'zip',
  txt: 'document',
  log: 'log',
  vue: 'vue',
  svelte: 'svelte',
  env: 'tune',
  woff: 'font',
  woff2: 'font',
  ttf: 'font',
  otf: 'font',
  mp3: 'audio',
  wav: 'audio',
  ogg: 'audio',
  flac: 'audio',
  mp4: 'video',
  webm: 'video',
  mov: 'video',
};

const FOLDERS: Record<string, string> = {
  src: 'folder-src',
  source: 'folder-src',
  lib: 'folder-src',
  docs: 'folder-docs',
  doc: 'folder-docs',
  test: 'folder-test',
  tests: 'folder-test',
  __tests__: 'folder-test',
  spec: 'folder-test',
  node_modules: 'folder-node',
  '.github': 'folder-github',
  config: 'folder-config',
  '.config': 'folder-config',
};

/** The icon name (a file in file-icons/, without .svg) for a tree entry. */
export function fileIconName(name: string, isDir: boolean, expanded = false): string {
  const lower = name.toLowerCase();
  if (isDir) {
    const base = FOLDERS[lower] ?? 'folder-base';
    return expanded ? `${base}-open` : base;
  }
  const special = BY_NAME[lower];
  if (special) return special;
  if (lower.startsWith('.env.')) return 'tune';
  const dot = lower.lastIndexOf('.');
  const ext = dot > 0 ? lower.slice(dot + 1) : '';
  if (lower.endsWith('.d.ts')) return 'typescript';
  return BY_EXTENSION[ext] ?? 'file';
}

const URLS = import.meta.glob<string>('./file-icons/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
});

export function fileIconUrl(name: string, isDir: boolean, expanded = false): string {
  const key = `./file-icons/${fileIconName(name, isDir, expanded)}.svg`;
  return URLS[key] ?? URLS['./file-icons/file.svg'] ?? '';
}
