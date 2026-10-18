// The Files dock panel: workspace tree with lazy folders, live watcher updates, keyboard
// navigation, a context menu and inline create/rename.
import './files.css';
import {
  createEffect,
  createMemo,
  createSignal,
  Match,
  on,
  onCleanup,
  Show,
  Switch,
} from 'solid-js';
import type { EntryKind } from '../../shared/types';
import { Menu, type MenuItem } from '../../shared/ui/Menu';
import type { IconName } from '../../shared/ui/Icon';
import { EmptyState } from '../../shared/ui/Surface';
import { t, type I18nKey } from '../i18n';
import {
  currentWorkspace,
  folderName,
  fsRoot,
  fsStatus,
  onFsChanges,
  openFolder,
} from '../workspace';
import { copyPath, createEntry, deleteEntry, renameEntry, type FileCallbacks } from './actions';
import type { TreeRow } from './treeModel';
import { treeKeyAction } from './treeKeys';
import { createTreeStore, targetFolder } from './treeStore';
import { TreeView, type DisplayRow, type TreeViewHandle } from './TreeView';
import { Button, IconButton } from '../../shared/ui/Button';

type Edit = { mode: 'rename'; path: string } | { mode: 'create'; parent: string; kind: EntryKind };

/** Entry rows plus, while creating, the input row at the top of the target folder. */
function withEditRow(rows: readonly TreeRow[], edit: Edit | null): DisplayRow[] {
  const display: DisplayRow[] = rows.map((row) => ({ type: 'entry', row }));
  if (edit?.mode !== 'create') return display;
  const parentIndex = rows.findIndex((row) => row.entry.path === edit.parent);
  const depth = parentIndex < 0 ? 0 : (rows[parentIndex]?.depth ?? 0) + 1;
  display.splice(parentIndex + 1, 0, { type: 'create', depth, kind: edit.kind });
  return display;
}

function ToolButton(props: { icon: IconName; label: I18nKey; onClick: () => void }) {
  return <IconButton icon={props.icon} label={t(props.label)} onClick={() => props.onClick()} />;
}

function NoWorkspace(props: { error: boolean }) {
  return (
    <EmptyState
      icon="folder-open"
      text={t(props.error ? 'files.unavailable' : 'files.noWorkspace')}
      action={
        <Button icon="folder-open" onClick={() => void openFolder()}>
          {t('workspace.open')}
        </Button>
      }
    />
  );
}

export function FilesPanel(props: FileCallbacks) {
  const tree = createTreeStore();
  const [selected, setSelected] = createSignal<string | null>(null);
  const [edit, setEdit] = createSignal<Edit | null>(null);
  const [menu, setMenu] = createSignal<{ x: number; y: number; row: TreeRow | null } | null>(null);
  const rows = createMemo(() => withEditRow(tree.rows(), edit()));
  let view: TreeViewHandle | undefined;

  createEffect(on(fsRoot, (root) => tree.reset(root)));
  onCleanup(onFsChanges((changes) => tree.applyChanges(changes)));

  const entryRows = () => tree.rows();
  const selectedIndex = () => entryRows().findIndex((row) => row.entry.path === selected());

  const select = (path: string | null) => {
    setSelected(path);
    const index = rows().findIndex((row) => row.type === 'entry' && row.row.entry.path === path);
    if (index >= 0) view?.scrollToIndex(index);
  };
  const activate = (row: TreeRow) => {
    if (row.entry.isDir) tree.toggle(row.entry.path);
    else props.onOpenFile(row.entry.path);
  };
  const startCreate = async (kind: EntryKind, row: TreeRow | undefined) => {
    const parent = targetFolder(row);
    if (parent) await tree.expand(parent);
    setEdit({ mode: 'create', parent, kind });
  };
  const finishEdit = async (name: string) => {
    const current = edit();
    setEdit(null);
    if (!current) return;
    const path =
      current.mode === 'create'
        ? await createEntry(tree, current.parent, name, current.kind, props)
        : await renameEntry(tree, current.path, name, props);
    if (path) select(path);
    view?.focus();
  };
  const cancelEdit = () => {
    setEdit(null);
    view?.focus();
  };
  const remove = async (row: TreeRow) => {
    if (await deleteEntry(tree, row.entry.path, row.entry.isDir)) setSelected(null);
    view?.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const action = treeKeyAction(event, entryRows(), selectedIndex(), view?.pageSize() ?? 10);
    const row = action ? entryRows()[action.index] : undefined;
    if (!action || !row) return;
    event.preventDefault();
    if (action.type === 'select') select(row.entry.path);
    else if (action.type === 'expand') void tree.expand(row.entry.path);
    else if (action.type === 'collapse') tree.collapse(row.entry.path);
    else if (action.type === 'activate') activate(row);
    else if (action.type === 'rename') setEdit({ mode: 'rename', path: row.entry.path });
    else void remove(row);
  };

  const menuItems = (row: TreeRow | null): MenuItem[] => [
    { id: 'newFile', label: t('files.newFile') },
    { id: 'newFolder', label: t('files.newFolder') },
    ...(row
      ? [
          { id: 'rename', label: t('files.rename') },
          { id: 'delete', label: t('files.delete') },
          { id: 'copyPath', label: t('files.copyPath') },
          { id: 'copyAbsolutePath', label: t('files.copyAbsolutePath') },
        ]
      : []),
  ];
  const onMenuSelect = (id: string, row: TreeRow | null) => {
    if (id === 'newFile' || id === 'newFolder') {
      void startCreate(id === 'newFile' ? 'file' : 'dir', row ?? undefined);
    } else if (!row) return;
    else if (id === 'rename') setEdit({ mode: 'rename', path: row.entry.path });
    else if (id === 'delete') void remove(row);
    else void copyPath(row.entry.path, id === 'copyAbsolutePath');
  };
  const selectedRow = () => entryRows()[selectedIndex()];
  const renaming = () => {
    const current = edit();
    return current?.mode === 'rename' ? current.path : null;
  };

  return (
    <div class="af-files">
      <Switch>
        <Match when={!currentWorkspace() || fsStatus() === 'error'}>
          <NoWorkspace error={fsStatus() === 'error'} />
        </Match>
        <Match when={currentWorkspace()}>
          {(root) => (
            <>
              <header class="af-files-header">
                <span class="af-files-title" title={root()}>
                  {folderName(root())}
                </span>
                <ToolButton
                  icon="file-plus"
                  label="files.newFile"
                  onClick={() => void startCreate('file', selectedRow())}
                />
                <ToolButton
                  icon="folder-plus"
                  label="files.newFolder"
                  onClick={() => void startCreate('dir', selectedRow())}
                />
                <ToolButton icon="refresh-cw" label="files.refresh" onClick={tree.refresh} />
                <ToolButton
                  icon="chevrons-down-up"
                  label="files.collapseAll"
                  onClick={tree.collapseAll}
                />
              </header>
              <TreeView
                ref={(handle) => (view = handle)}
                rows={rows()}
                selected={selected()}
                renaming={renaming()}
                onRowClick={(row) => {
                  setSelected(row.entry.path);
                  activate(row);
                }}
                onContextMenu={(row, event) => {
                  event.preventDefault();
                  if (row) setSelected(row.entry.path);
                  setMenu({ x: event.clientX, y: event.clientY, row });
                }}
                onKeyDown={onKeyDown}
                onCommit={(name) => void finishEdit(name)}
                onCancel={cancelEdit}
              />
            </>
          )}
        </Match>
      </Switch>
      <Show when={menu()}>
        {(open) => (
          <Menu
            anchor={{ x: open().x, y: open().y }}
            placement="below-end"
            items={menuItems(open().row)}
            onSelect={(id) => onMenuSelect(id, open().row)}
            onClose={() => setMenu(null)}
          />
        )}
      </Show>
    </div>
  );
}
