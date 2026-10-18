// Ctrl/Cmd+P palette: fuzzy file search over the workspace (fs_quick_open). A query that
// starts with `>` lists the commands the app passes in instead (e.g. "Open Agents library").
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  on,
  onCleanup,
  onMount,
  Show,
} from 'solid-js';
import { Portal } from 'solid-js/web';
import { debounce } from '../../shared/debounce';
import { ipc } from '../../shared/ipc';
import type { QuickOpenMatch } from '../../shared/types';
import { FileIcon } from '../../shared/ui/FileIcon';
import { reportError, t } from '../i18n';
import { baseName, fsRoot, openFolder } from '../workspace';
import { highlight, type Segment } from './highlight';
import { closeQuickOpen, quickOpenVisible } from './quickOpenStore';
import { Button } from '../../shared/ui/Button';
import { Input } from '../../shared/ui/Field';

const SEARCH_DELAY_MS = 40;

export interface PaletteCommand {
  id: string;
  label: string;
  run: () => void;
}

function Marked(props: { segments: Segment[] }) {
  return (
    <For each={props.segments}>
      {(segment) => (segment.match ? <mark>{segment.text}</mark> : <>{segment.text}</>)}
    </For>
  );
}

function Palette(props: {
  onOpen: (path: string) => void;
  commands: () => readonly PaletteCommand[];
}) {
  const [query, setQuery] = createSignal('');
  const [results, setResults] = createSignal<readonly QuickOpenMatch[]>([]);
  const [active, setActive] = createSignal(0);
  let input: HTMLInputElement | undefined;
  let list: HTMLUListElement | undefined;
  let request = 0;
  let disposed = false;

  const commandMode = () => query().startsWith('>');
  const commands = createMemo(() => {
    const words = query().slice(1).trim().toLowerCase();
    return props.commands().filter((c) => c.label.toLowerCase().includes(words));
  });
  const count = () => (commandMode() ? commands().length : results().length);

  const search = debounce(() => {
    const ticket = ++request;
    if (!fsRoot() || commandMode()) return;
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
  createEffect(on(commandMode, () => setActive(0)));
  onMount(() => input?.focus());
  onCleanup(() => (disposed = true));

  const choose = (match: QuickOpenMatch | undefined) => {
    if (!match) return;
    closeQuickOpen();
    props.onOpen(match.path);
  };
  const run = (command: PaletteCommand | undefined) => {
    if (!command) return;
    closeQuickOpen();
    command.run();
  };
  const move = (delta: number) => {
    const total = count();
    if (total === 0) return;
    const next = (active() + delta + total) % total;
    setActive(next);
    list?.children[next]?.scrollIntoView({ block: 'nearest' });
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      move(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (commandMode()) run(commands()[active()]);
      else choose(results()[active()]);
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
              <Button
                size="sm"
                onClick={() => {
                  closeQuickOpen();
                  void openFolder();
                }}
              >
                {t('workspace.open')}
              </Button>
            </div>
          }
        >
          <Input
            class="af-quickopen-input"
            ref={input}
            type="text"
            spellcheck={false}
            autocomplete="off"
            placeholder={t('quickOpen.placeholder')}
            aria-label={t('quickOpen.title')}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={onKeyDown}
          />
          <Show when={commandMode()}>
            <ul ref={list} class="af-quickopen-list" role="listbox">
              <For
                each={commands()}
                fallback={<li class="af-quickopen-none">{t('quickOpen.noCommands')}</li>}
              >
                {(command, index) => (
                  <li
                    class="af-quickopen-item"
                    classList={{ active: index() === active() }}
                    role="option"
                    aria-selected={index() === active()}
                    onPointerMove={() => setActive(index())}
                    onClick={() => run(command)}
                  >
                    <span class="af-quickopen-name">{command.label}</span>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <ul ref={list} class="af-quickopen-list" role="listbox" hidden={commandMode()}>
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
                    <FileIcon name={baseName(match.path)} />
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
export function QuickOpen(props: {
  onOpen: (path: string) => void;
  commands?: () => readonly PaletteCommand[];
}) {
  return (
    <Show when={quickOpenVisible()}>
      <Portal>
        <Palette onOpen={props.onOpen} commands={props.commands ?? (() => [])} />
      </Portal>
    </Show>
  );
}
