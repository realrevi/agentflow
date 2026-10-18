// Virtualized tree: only the rows in view (plus a little overscan) are in the DOM, so
// folders with 10k+ entries scroll smoothly. Scroll position survives tree updates.
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { FILE_PATH_MIME } from '../../shared/dnd';
import type { EntryKind } from '../../shared/types';
import { FileIcon } from '../../shared/ui/FileIcon';
import { Icon } from '../../shared/ui/Icon';
import { t } from '../i18n';
import { currentWorkspace } from '../workspace';
import { agentChanged } from './agentMarks';
import { NameInput } from './NameInput';
import type { TreeRow } from './treeModel';

export const ROW_HEIGHT = 22;
const OVERSCAN = 10;
const INDENT = 12;

export type DisplayRow =
  { type: 'entry'; row: TreeRow } | { type: 'create'; depth: number; kind: EntryKind };

export interface TreeViewHandle {
  scrollToIndex(index: number): void;
  focus(): void;
  pageSize(): number;
}

interface TreeViewProps {
  rows: readonly DisplayRow[];
  selected: string | null;
  renaming: string | null;
  onRowClick: (row: TreeRow) => void;
  onContextMenu: (row: TreeRow | null, event: MouseEvent) => void;
  onKeyDown: (event: KeyboardEvent) => void;
  onCommit: (name: string) => void;
  onCancel: () => void;
  ref: (handle: TreeViewHandle) => void;
}

export function TreeView(props: TreeViewProps) {
  let scroller: HTMLDivElement | undefined;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [height, setHeight] = createSignal(0);

  const range = createMemo(() => {
    const start = Math.max(0, Math.floor(scrollTop() / ROW_HEIGHT) - OVERSCAN);
    const end = Math.min(
      props.rows.length,
      Math.ceil((scrollTop() + height()) / ROW_HEIGHT) + OVERSCAN,
    );
    return { start, end };
  });
  const windowed = createMemo(() => props.rows.slice(range().start, range().end));

  onMount(() => {
    if (!scroller) return;
    const observer = new ResizeObserver(() => setHeight(scroller?.clientHeight ?? 0));
    observer.observe(scroller);
    onCleanup(() => observer.disconnect());
  });

  props.ref({
    scrollToIndex(index) {
      if (!scroller) return;
      const top = index * ROW_HEIGHT;
      if (top < scroller.scrollTop) scroller.scrollTop = top;
      else if (top + ROW_HEIGHT > scroller.scrollTop + scroller.clientHeight) {
        scroller.scrollTop = top + ROW_HEIGHT - scroller.clientHeight;
      }
    },
    focus: () => scroller?.focus(),
    pageSize: () => Math.max(1, Math.floor((scroller?.clientHeight ?? 0) / ROW_HEIGHT) - 1),
  });

  return (
    <div
      ref={scroller}
      class="af-tree"
      role="tree"
      tabindex="0"
      aria-label={t('panel.files.title')}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      onKeyDown={(event) => props.onKeyDown(event)}
      onContextMenu={(event) => {
        if (event.target === event.currentTarget || event.target === scroller?.firstChild) {
          props.onContextMenu(null, event);
        }
      }}
    >
      <div class="af-tree-content" style={{ height: `${props.rows.length * ROW_HEIGHT}px` }}>
        <For each={windowed()}>
          {(item, i) => {
            const top = () => `${(range().start + i()) * ROW_HEIGHT}px`;
            if (item.type === 'create') {
              return (
                <div class="af-tree-row" style={{ top: top(), 'padding-left': indent(item.depth) }}>
                  <span class="af-tree-twisty" />
                  <FileIcon name="" isDir={item.kind === 'dir'} class="af-tree-icon" />
                  <NameInput
                    initial=""
                    label={t(item.kind === 'dir' ? 'files.newFolder' : 'files.newFile')}
                    onCommit={(name) => props.onCommit(name)}
                    onCancel={() => props.onCancel()}
                  />
                </div>
              );
            }
            const { entry, depth, expanded } = item.row;
            return (
              <div
                class="af-tree-row"
                classList={{ selected: props.selected === entry.path }}
                role="treeitem"
                aria-level={depth + 1}
                aria-expanded={entry.isDir ? expanded : undefined}
                aria-selected={props.selected === entry.path}
                title={entry.path}
                draggable={props.renaming !== entry.path}
                style={{ top: top(), 'padding-left': indent(depth) }}
                onClick={() => props.onRowClick(item.row)}
                onContextMenu={(event) => {
                  event.preventDefault();
                  props.onContextMenu(item.row, event);
                }}
                onDragStart={(event) => {
                  event.dataTransfer?.setData(FILE_PATH_MIME, entry.path);
                  event.dataTransfer?.setData('text/plain', entry.path);
                  if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
                }}
              >
                <span class="af-tree-twisty" classList={{ open: expanded }}>
                  <Show when={entry.isDir}>
                    <Icon name="chevron-right" size={14} />
                  </Show>
                </span>
                <FileIcon
                  name={entry.name}
                  isDir={entry.isDir}
                  expanded={expanded}
                  class="af-tree-icon"
                />
                <Show
                  when={props.renaming === entry.path}
                  fallback={
                    <span class="af-tree-name" classList={{ link: entry.isSymlink }}>
                      {entry.name}
                      <Show when={!entry.isDir && agentChanged(currentWorkspace(), entry.path)}>
                        <span
                          class="af-tree-agent-mark"
                          role="img"
                          aria-label={t('files.agentChanged')}
                          title={t('files.agentChanged')}
                        />
                      </Show>
                    </span>
                  }
                >
                  <NameInput
                    initial={entry.name}
                    label={t('files.rename')}
                    onCommit={(name) => props.onCommit(name)}
                    onCancel={() => props.onCancel()}
                  />
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}

const indent = (depth: number) => `${4 + depth * INDENT}px`;
