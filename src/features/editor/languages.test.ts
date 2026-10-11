import { describe, expect, it } from 'vitest';
import { languageFor } from './languages';

describe('languageFor', () => {
  it('maps extensions to languages', () => {
    const cases: [string, string][] = [
      ['README.md', 'markdown'],
      ['package.json', 'json'],
      ['tsconfig.jsonc', 'json'],
      ['main.py', 'python'],
      ['app.js', 'javascript'],
      ['App.jsx', 'jsx'],
      ['index.ts', 'typescript'],
      ['View.TSX', 'tsx'],
      ['lib.rs', 'rust'],
      ['index.html', 'html'],
      ['site.css', 'css'],
      ['main.c', 'cpp'],
      ['vec.hpp', 'cpp'],
      ['Program.cs', 'csharp'],
      ['ci.yml', 'yaml'],
      ['Cargo.toml', 'toml'],
    ];
    for (const [file, language] of cases) expect(languageFor(file), file).toBe(language);
  });

  it('falls back to plain text', () => {
    expect(languageFor('Makefile')).toBe('plain');
    expect(languageFor('.gitignore')).toBe('plain');
    expect(languageFor('notes.txt')).toBe('plain');
  });
});
