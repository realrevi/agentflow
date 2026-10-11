// File extension -> editor language. The grammars themselves load on demand
// (loadLanguage.ts), each in its own chunk.
import { extension } from '../workspace';

export type LanguageId =
  | 'markdown'
  | 'json'
  | 'python'
  | 'javascript'
  | 'jsx'
  | 'typescript'
  | 'tsx'
  | 'rust'
  | 'html'
  | 'css'
  | 'cpp'
  | 'csharp'
  | 'yaml'
  | 'toml'
  | 'plain';

const BY_EXTENSION: Record<string, LanguageId> = {
  md: 'markdown',
  markdown: 'markdown',
  json: 'json',
  jsonc: 'json',
  py: 'python',
  pyw: 'python',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'jsx',
  ts: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  tsx: 'tsx',
  rs: 'rust',
  html: 'html',
  htm: 'html',
  css: 'css',
  c: 'cpp',
  h: 'cpp',
  cc: 'cpp',
  cpp: 'cpp',
  cxx: 'cpp',
  hh: 'cpp',
  hpp: 'cpp',
  hxx: 'cpp',
  cs: 'csharp',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
};

export function languageFor(fileName: string): LanguageId {
  return BY_EXTENSION[extension(fileName)] ?? 'plain';
}
