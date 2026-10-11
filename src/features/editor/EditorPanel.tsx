// Lazy-loaded editor panel (see layout/renderers.tsx): CodeMirror 6 for one workspace file,
// with save, a dirty dot, disk-change banners and info views. Default export for `lazy()`.
import './editor.css';
import type { DockviewPanelApi } from 'dockview-core';
import {
  createEffect,
  createSignal,
  Match,
  onCleanup,
  onMount,
  Show,
  Switch,
  type JSX,
} from 'solid-js';
import { explorerIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { activeTheme } from '../theme';
import { currentWorkspace, fsRoot, onFsChanges } from '../workspace';
import { EditorController, type DiskBanner, type EditorInfo } from './EditorController';
import { registerEditor, setEditorDirty } from './editorStore';

export interface EditorPanelProps {
  panelId: string;
  api: DockviewPanelApi;
  /** Workspace folder the file belongs to; `path` is relative to it. */
  root: string;
  path: string;
}

const BANNER_TEXT: Record<Exclude<DiskBanner, null>, I18nKey> = {
  changed: 'editor.banner.changed',
  conflict: 'editor.banner.conflict',
  deleted: 'editor.banner.deleted',
};

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function InfoView(props: { title: I18nKey; body: string; children?: JSX.Element }) {
  return (
    <div class="af-editor-info">
      <PixelIcon art={explorerIcon} scale={4} />
      <h2 class="af-pixel-title">{t(props.title)}</h2>
      <p>{props.body}</p>
      {props.children}
    </div>
  );
}

export default function EditorPanel(props: EditorPanelProps) {
  const [info, setInfo] = createSignal<EditorInfo>({ status: 'loading' });
  const [banner, setBanner] = createSignal<DiskBanner>(null);
  const [path, setPath] = createSignal(props.path);
  let host: HTMLDivElement | undefined;
  let controller: EditorController | undefined;

  // Only this file's workspace may be read or written; other workspaces show an overlay.
  const ownWorkspace = () => currentWorkspace() === props.root;
  const canUseFiles = () => fsRoot() === props.root;

  onMount(() => {
    if (!host) return;
    const editor = new EditorController({
      host,
      path: props.path,
      dark: activeTheme()?.kind !== 'light',
      canUseFiles,
      onState: setInfo,
      onBanner: setBanner,
      onDirty: (dirty) => setEditorDirty(props.panelId, dirty),
    });
    controller = editor;
    const unregister = registerEditor(props.panelId, {
      fileName: () => editor.fileName(),
      save: () => editor.save(),
    });
    const subscriptions = [
      props.api.onDidParametersChange((params) => {
        const next = (params as { path?: unknown }).path;
        if (typeof next === 'string' && next !== path()) {
          setPath(next);
          editor.setPath(next);
        }
      }),
      props.api.onDidActiveChange((event) => event.isActive && editor.focus()),
      props.api.onDidVisibilityChange((event) => event.isVisible && editor.refresh()),
    ];
    const unsubscribe = onFsChanges((changes) => {
      if (canUseFiles() && (changes.rescan || changes.paths.includes(path()))) {
        void editor.checkDisk();
      }
    });
    onCleanup(() => {
      unsubscribe();
      for (const subscription of subscriptions) subscription.dispose();
      unregister();
      editor.dispose();
    });
  });

  // Open once the backend serves this workspace; re-check after switching back to it.
  let opened = false;
  createEffect(() => {
    if (!canUseFiles() || !controller) return;
    if (opened) {
      void controller.checkDisk();
      return;
    }
    opened = true;
    void controller.load().then(() => props.api.isActive && controller?.focus());
  });
  createEffect(() => controller?.setDark(activeTheme()?.kind !== 'light'));
  createEffect(() => {
    host?.style.setProperty('--af-editor-font-size', `${settings.terminal.fontSize}px`);
    controller?.refresh();
  });

  const size = () => formatSize(info().size ?? 0);

  return (
    <div class="af-editor">
      <Show when={banner()}>
        {(kind) => (
          <div class="af-editor-banner" role="alert">
            <span>{t(BANNER_TEXT[kind()])}</span>
            <Show when={kind() !== 'deleted'}>
              <button
                type="button"
                class="af-button small"
                onClick={() => void controller?.reload()}
              >
                {t('editor.banner.reload')}
              </button>
            </Show>
            <button
              type="button"
              class="af-button small secondary"
              onClick={() => void controller?.keepMine()}
            >
              {t('editor.banner.keepMine')}
            </button>
          </div>
        )}
      </Show>
      <Show when={info().readOnly}>
        <div class="af-editor-note">{t('editor.readOnlyLossy')}</div>
      </Show>
      <div ref={host} class="af-editor-host" classList={{ hidden: info().status !== 'ready' }} />
      <Switch>
        <Match when={info().status === 'loading'}>
          <div class="af-panel-loading">{t('panel.loading')}</div>
        </Match>
        <Match when={info().status === 'binary'}>
          <InfoView title="editor.binary.title" body={t('editor.binary.body', { size: size() })} />
        </Match>
        <Match when={info().status === 'tooLarge'}>
          <InfoView
            title="editor.tooLarge.title"
            body={t('editor.tooLarge.body', { size: size() })}
          />
        </Match>
        <Match when={info().status === 'error'}>
          <InfoView title="editor.error.title" body={info().message ?? ''}>
            <button type="button" class="af-button" onClick={() => void controller?.load()}>
              {t('editor.retry')}
            </button>
          </InfoView>
        </Match>
      </Switch>
      <Show when={!ownWorkspace()}>
        <div class="af-editor-overlay">
          <InfoView
            title="editor.otherWorkspace.title"
            body={t('editor.otherWorkspace.body', { path: props.root })}
          />
        </div>
      </Show>
    </div>
  );
}
