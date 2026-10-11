// Loads a language on demand; each grammar is its own chunk.
import { StreamLanguage } from '@codemirror/language';
import type { Extension } from '@codemirror/state';
import type { LanguageId } from './languages';

export async function loadLanguage(id: LanguageId): Promise<Extension> {
  switch (id) {
    case 'markdown':
      return (await import('@codemirror/lang-markdown')).markdown();
    case 'json':
      return (await import('@codemirror/lang-json')).json();
    case 'python':
      return (await import('@codemirror/lang-python')).python();
    case 'javascript':
    case 'jsx':
    case 'typescript':
    case 'tsx': {
      const { javascript } = await import('@codemirror/lang-javascript');
      return javascript({
        jsx: id === 'jsx' || id === 'tsx',
        typescript: id === 'typescript' || id === 'tsx',
      });
    }
    case 'rust':
      return (await import('@codemirror/lang-rust')).rust();
    case 'html':
      return (await import('@codemirror/lang-html')).html();
    case 'css':
      return (await import('@codemirror/lang-css')).css();
    case 'cpp':
      return (await import('@codemirror/lang-cpp')).cpp();
    case 'csharp':
      return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/clike')).csharp);
    case 'yaml':
      return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/yaml')).yaml);
    case 'toml':
      return StreamLanguage.define((await import('@codemirror/legacy-modes/mode/toml')).toml);
    case 'plain':
      return [];
  }
}
