import { describe, expect, it } from 'vitest';
import { fileIconName } from './fileIconNames';

describe('file icons', () => {
  it('picks icons by extension, case-insensitive', () => {
    expect(fileIconName('main.ts', false)).toBe('typescript');
    expect(fileIconName('App.TSX', false)).toBe('react_ts');
    expect(fileIconName('lib.rs', false)).toBe('rust');
    expect(fileIconName('script.py', false)).toBe('python');
    expect(fileIconName('styles.scss', false)).toBe('sass');
    expect(fileIconName('logo.png', false)).toBe('image');
    expect(fileIconName('types.d.ts', false)).toBe('typescript');
  });

  it('knows special file names before extensions', () => {
    expect(fileIconName('package.json', false)).toBe('nodejs');
    expect(fileIconName('Cargo.toml', false)).toBe('rust');
    expect(fileIconName('Cargo.lock', false)).toBe('lock');
    expect(fileIconName('Dockerfile', false)).toBe('docker');
    expect(fileIconName('README.md', false)).toBe('readme');
    expect(fileIconName('LICENSE', false)).toBe('license');
    expect(fileIconName('.gitignore', false)).toBe('git');
    expect(fileIconName('.env.local', false)).toBe('tune');
    expect(fileIconName('pnpm-lock.yaml', false)).toBe('pnpm');
  });

  it('falls back to a generic file and handles folders', () => {
    expect(fileIconName('notes.xyz', false)).toBe('file');
    expect(fileIconName('Makefile', false)).toBe('makefile');
    expect(fileIconName('noext', false)).toBe('file');
    expect(fileIconName('src', true)).toBe('folder-src');
    expect(fileIconName('src', true, true)).toBe('folder-src-open');
    expect(fileIconName('anything', true)).toBe('folder-base');
    expect(fileIconName('anything', true, true)).toBe('folder-base-open');
  });
});
