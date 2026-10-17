// A read-only unified diff (before -> after) for agent permission cards. Lazy chunk
// (CodeMirror + @codemirror/merge); the layout hands it to chat panels. A new file
// (`before: null`) shows as all added. Colors come from the theme (`--af-diff-*`).
import { unifiedMergeView } from '@codemirror/merge';
import { syntaxHighlighting } from '@codemirror/language';
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView, lineNumbers } from '@codemirror/view';
import { onCleanup, onMount } from 'solid-js';
import { languageFor } from './languages';
import { loadLanguage } from './loadLanguage';
import { editorTheme, highlightStyle } from './theme';

export interface DiffViewProps {
  path: string;
  before: string | null;
  after: string;
}

const diffTheme = EditorView.theme({
  '&': { height: 'auto', maxHeight: '320px', fontSize: '12px' },
  '.cm-scroller': { overflow: 'auto' },
  '.cm-changedLine, .cm-insertedLine': { backgroundColor: 'var(--af-diff-inserted)' },
  '.cm-changedText': { background: 'none' },
  '.cm-deletedChunk, .cm-deletedLine': { backgroundColor: 'var(--af-diff-removed)' },
  '.cm-deletedChunk del, .cm-deletedText': { textDecoration: 'none', background: 'none' },
  '.cm-changeGutter': { width: '3px', paddingLeft: '1px' },
  '.cm-changedLineGutter': { background: 'var(--af-success)' },
  '.cm-deletedLineGutter': { background: 'var(--af-error)' },
});

export default function DiffView(props: DiffViewProps) {
  let host: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  const language = new Compartment();

  onMount(() => {
    if (!host) return;
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: props.after,
        extensions: [
          lineNumbers(),
          EditorState.readOnly.of(true),
          EditorView.editable.of(false),
          editorTheme,
          diffTheme,
          syntaxHighlighting(highlightStyle),
          language.of([]),
          unifiedMergeView({
            original: props.before ?? '',
            mergeControls: false,
            syntaxHighlightDeletions: true,
            collapseUnchanged: { margin: 3, minSize: 6 },
          }),
        ],
      }),
    });
    void loadLanguage(languageFor(props.path)).then((extension) => {
      view?.dispatch({ effects: language.reconfigure(extension) });
    });
  });
  onCleanup(() => view?.destroy());

  return <div ref={host} class="af-diff-view" />;
}
