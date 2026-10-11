import { describe, expect, it } from 'vitest';
import {
  absolutePath,
  baseName,
  extension,
  isInside,
  joinPath,
  parentPath,
  renamedPath,
} from './paths';

describe('workspace paths', () => {
  it('splits and joins relative paths', () => {
    expect(parentPath('src/main.rs')).toBe('src');
    expect(parentPath('README.md')).toBe('');
    expect(baseName('src/ui/App.tsx')).toBe('App.tsx');
    expect(joinPath('', 'a.txt')).toBe('a.txt');
    expect(joinPath('src', 'a.txt')).toBe('src/a.txt');
  });

  it('knows what is inside a folder', () => {
    expect(isInside('src/a.ts', 'src')).toBe(true);
    expect(isInside('src', 'src')).toBe(true);
    expect(isInside('srcfoo/a.ts', 'src')).toBe(false);
    expect(isInside('anything', '')).toBe(true);
  });

  it('follows renames of files and folders', () => {
    expect(renamedPath('src/a.ts', 'src/a.ts', 'src/b.ts')).toBe('src/b.ts');
    expect(renamedPath('src/ui/x.ts', 'src', 'lib')).toBe('lib/ui/x.ts');
    expect(renamedPath('srcfoo/x.ts', 'src', 'lib')).toBeNull();
  });

  it('reads extensions', () => {
    expect(extension('Main.RS')).toBe('rs');
    expect(extension('archive.tar.gz')).toBe('gz');
    expect(extension('.gitignore')).toBe('');
    expect(extension('Makefile')).toBe('');
  });

  it('builds absolute paths with the native separator', () => {
    expect(absolutePath('/home/me/app', 'src/main.rs')).toBe('/home/me/app/src/main.rs');
    expect(absolutePath('/home/me/app/', 'a')).toBe('/home/me/app/a');
    expect(absolutePath('C:\\code\\app', 'src/main.rs')).toBe('C:\\code\\app\\src\\main.rs');
    expect(absolutePath('C:\\', 'a.txt')).toBe('C:\\a.txt');
    expect(absolutePath('/home/me/app', '')).toBe('/home/me/app');
  });
});
