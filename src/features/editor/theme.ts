// CodeMirror styling from AgentFlow theme variables. Everything is a CSS variable, so a
// theme switch restyles open editors instantly without reconfiguring them.
import { HighlightStyle } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

const FONT_FAMILY = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";
const syntax = (name: string) => `var(--af-syntax-${name})`;

export const editorTheme = EditorView.theme({
  '&': {
    height: '100%',
    color: 'var(--af-fg)',
    backgroundColor: 'var(--af-bg)',
    fontSize: 'var(--af-editor-font-size, 13px)',
  },
  '.cm-scroller': { fontFamily: FONT_FAMILY, lineHeight: '1.55' },
  '.cm-content': { caretColor: 'var(--af-editor-cursor)' },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: 'var(--af-editor-cursor)',
    borderLeftWidth: '2px',
  },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection':
    { backgroundColor: 'var(--af-selection)' },
  '.cm-activeLine': { backgroundColor: 'var(--af-editor-line-highlight)' },
  '.cm-gutters': {
    backgroundColor: 'var(--af-bg)',
    color: 'var(--af-editor-gutter-fg)',
    border: 'none',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'var(--af-editor-line-highlight)',
    color: 'var(--af-editor-gutter-active-fg)',
  },
  '.cm-selectionMatch': { backgroundColor: 'var(--af-editor-selection-match)' },
  '.cm-searchMatch': { backgroundColor: 'var(--af-editor-find-match)' },
  '.cm-searchMatch.cm-searchMatch-selected': { outline: '1px solid var(--af-accent)' },
  '&.cm-focused .cm-matchingBracket': {
    backgroundColor: 'transparent',
    outline: '1px solid var(--af-editor-bracket)',
  },
  '.cm-foldPlaceholder': {
    backgroundColor: 'var(--af-list-hover-bg)',
    border: 'none',
    color: 'var(--af-fg-muted)',
  },
  '.cm-panels': {
    backgroundColor: 'var(--af-widget-bg)',
    color: 'var(--af-widget-fg)',
  },
  '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--af-widget-border)' },
  '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--af-widget-border)' },
  '.cm-textfield': {
    backgroundColor: 'var(--af-input-bg)',
    color: 'var(--af-input-fg)',
    border: '1px solid var(--af-input-border)',
    borderRadius: '4px',
  },
  '.cm-button': {
    backgroundImage: 'none',
    backgroundColor: 'var(--af-button-secondary-bg)',
    color: 'var(--af-button-secondary-fg)',
    border: '1px solid var(--af-input-border)',
    borderRadius: '4px',
  },
  '.cm-tooltip': {
    backgroundColor: 'var(--af-widget-bg)',
    color: 'var(--af-widget-fg)',
    border: '1px solid var(--af-widget-border)',
  },
  '.cm-specialChar': { color: 'var(--af-warning)' },
});

export const highlightStyle = HighlightStyle.define([
  {
    tag: [tags.keyword, tags.modifier, tags.operatorKeyword, tags.controlKeyword],
    color: syntax('keyword'),
  },
  {
    tag: [tags.string, tags.special(tags.string), tags.regexp, tags.character],
    color: syntax('string'),
  },
  { tag: [tags.number, tags.integer, tags.float], color: syntax('number') },
  {
    tag: [tags.bool, tags.null, tags.atom, tags.constant(tags.variableName), tags.self],
    color: syntax('constant'),
  },
  {
    tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment],
    color: syntax('comment'),
    fontStyle: 'italic',
  },
  {
    tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.macroName],
    color: syntax('function'),
  },
  {
    tag: [tags.typeName, tags.className, tags.namespace, tags.standard(tags.typeName)],
    color: syntax('type'),
  },
  { tag: [tags.propertyName, tags.labelName], color: syntax('property') },
  { tag: [tags.tagName, tags.angleBracket], color: syntax('tag') },
  { tag: tags.attributeName, color: syntax('attribute') },
  { tag: [tags.variableName, tags.definition(tags.variableName)], color: syntax('variable') },
  { tag: [tags.meta, tags.annotation, tags.processingInstruction], color: syntax('meta') },
  { tag: tags.heading, color: syntax('heading'), fontWeight: 'bold' },
  { tag: [tags.link, tags.url], color: 'var(--af-link)', textDecoration: 'underline' },
  { tag: tags.emphasis, fontStyle: 'italic' },
  { tag: tags.strong, fontWeight: 'bold' },
  { tag: tags.strikethrough, textDecoration: 'line-through' },
  { tag: tags.invalid, color: 'var(--af-error)' },
]);
