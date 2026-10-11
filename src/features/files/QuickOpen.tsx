// Ctrl/Cmd+P palette: fuzzy file search over the workspace (fs_quick_open).
import { createEffect, createSignal, For, on, onCleanup, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { debounce } from '../../shared/debounce';
import { ipc } from '../../shared/ipc';
import type { QuickOpenMatch } from '../../shared/types';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { reportError, t } from '../i18n';
import { baseName, fsRoot, openFolder } from '../workspace';
import { iconFor } from './fileIcons';
import { highlight, type Segment } from './highlight';
import { closeQuickOpen, quickOpenVisible } from './quickOpenStore';

const SEARCH_DELAY_MS = 40;

function Marked(props: { segments: Segment[] }) {
  return (
    <For each={props.segments}>
      {(segment) => (segment.match ? <mark>{segment.text}</mark> : <>{segment.text}</>)}
    </For>
  );
}

function Palette(props: { onOpen: (path: string) => void }) {
  const [query, setQuery] = createSignal('');
  const [results, setResults] = createSignal<readonly QuickOpenMatch[]>([]);
  const [active, setActive] = createSignal(0);
  let input: HTMLInputElement | undefined;
  let list: HTMLUListElement | undefined;
  let request = 0;
  let disposed = false;

  const search = debounce(() => {
    const ticket = ++request;
    if (!fsRoot()) return;
    ipc.fsQuickOpen(query()).then(
      (found) => {
        if (disposed || ticket !== request) return;
        setResults(found);
        setActive(0);
      },
      (err: unknown) => !disposed && reportError('error.quickOpen', err),
    );
  }, SEARCH_DELAY_MS);
  createEffect(on([query, fsRoot], () => search()));
  onMount(() => input?.focus());
  onCleanup(() => (disposed = true));

  const choose = (match: QuickOpenMatch | undefined) => {
    if (!match) return;
    closeQuickOpen();
    props.onOpen(match.path);
  };
  const move = (delta: number) => {
    const count = results().length;
    if (count === 0) return;
    const next = (active() + delta + count) % count;
    setActive(next);
    list?.children[next]?.scrollIntoView({ block: 'nearest' });
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      move(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      choose(results()[active()]);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      closeQuickOpen();
    }
  };

  return (
    <div
      class="af-quickopen-backdrop"
      onPointerDown={(event) => event.target === event.currentTarget && closeQuickOpen()}
    >
      <div class="af-quickopen" role="dialog" aria-label={t('quickOpen.title')}>
        <Show
          when={fsRoot()}
          fallback={
            <div class="af-quickopen-empty">
              <p>{t('files.noWorkspace')}</p>
              <button
                type="button"
                class="af-button small"
                onClick={() => {
                  closeQuickOpen();
                  void openFolder();
                }}
              >
                {t('workspace.open')}
              </button>
            </div>
          }
        >
          <input
            ref={input}
            class="af-input af-quickopen-input"
            type="text"
            spellcheck={false}
            autocomplete="off"
            placeholder={t('quickOpen.placeholder')}
            aria-label={t('quickOpen.title')}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={onKeyDown}
          />
          <ul ref={list} class="af-quickopen-list" role="listbox">
            <For
              each={results()}
              fallback={<li class="af-quickopen-none">{t('quickOpen.noResults')}</li>}
            >
              {(match, index) => {
                const parts = highlight(match.path, match.positions);
                return (
                  <li
                    class="af-quickopen-item"
                    classList={{ active: index() === active() }}
                    role="option"
                    aria-selected={index() === active()}
                    title={match.path}
                    onPointerMove={() => setActive(index())}
                    onClick={() => choose(match)}
                  >
                    <PixelIcon art={iconFor(baseName(match.path), false)} />
                    <span class="af-quickopen-name">
                      <Marked segments={parts.name} />
                    </span>
                    <span class="af-quickopen-folder">
                      <Marked segments={parts.folder} />
                    </span>
                  </li>
                );
              }}
            </For>
          </ul>
        </Show>
      </div>
    </div>
  );
}

/** Mounted once by the app; shows the palette while it is open. */
export function QuickOpen(props: { onOpen: (path: string) => void }) {
  return (
    <Show when={quickOpenVisible()}>
      <Portal>
        <Palette onOpen={props.onOpen} />
      </Portal>
    </Show>
  );
}
