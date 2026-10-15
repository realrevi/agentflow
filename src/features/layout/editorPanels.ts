// Editor tabs: open (or focus) a workspace file, and follow renames from the file tree.
import type { DockviewApi, IDockviewPanel } from 'dockview-core';
import { currentWorkspace, renamedPath } from '../workspace';
import { addPanel, currentDock, nextIndex, syncOpenPanels } from './layoutStore';
import { panelTitle, parsePanelParams } from './panels';

function editorPanels(dock: DockviewApi): { panel: IDockviewPanel; root: string; path: string }[] {
  return dock.panels.flatMap((panel) => {
    const params = parsePanelParams(panel.params);
    return params?.kind === 'editor' ? [{ panel, root: params.root, path: params.path }] : [];
  });
}

/** Opens a workspace file in an editor tab in the center (or focuses its tab). */
export function openEditor(path: string): void {
  const dock = currentDock();
  const root = currentWorkspace();
  if (!dock || !root) return;
  const existing = editorPanels(dock).find(
    (editor) => editor.root === root && editor.path === path,
  );
  if (existing) existing.panel.api.setActive();
  else addPanel(dock, { kind: 'editor', index: nextIndex(dock, 'editor'), root, path });
}

/** Keeps editor tabs pointing at files that were renamed or moved in the file tree. */
export function renameEditors(from: string, to: string): void {
  const dock = currentDock();
  const root = currentWorkspace();
  if (!dock || !root) return;
  for (const editor of editorPanels(dock)) {
    const path = editor.root === root ? renamedPath(editor.path, from, to) : null;
    if (!path) continue;
    const params = parsePanelParams(editor.panel.params);
    if (params?.kind !== 'editor') continue;
    editor.panel.api.updateParameters({ ...params, path });
    editor.panel.api.setTitle(panelTitle({ ...params, path }));
  }
  syncOpenPanels();
}
