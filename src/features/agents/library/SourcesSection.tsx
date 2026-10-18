// The library's sources: each one's status (imported and unsupported counts, last check,
// errors, the unsupported files with their reasons), the user's own unverified sources,
// the daily refresh switch and the optional GitHub token (OS keychain, write-only).
import { createResource, createSignal, For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { LibraryCatalog } from '../../../shared/types';
import { reportError, t } from '../../i18n';
import {
  flushSettings,
  settings,
  updateSettings,
  type ReactiveSettings,
} from '../../settings/settingsStore';
import { TrustBadge } from './ItemCard';
import { errorText, reasonText } from './labels';
import { refreshLibrary } from './libraryStore';

const GITHUB_URL =
  /^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(\/(tree|blob)\/.+)?\/?$/;

function TokenField() {
  const [status, { refetch }] = createResource(() => ipc.libraryTokenStatus().catch(() => null));
  const [token, setToken] = createSignal('');
  const save = async () => {
    try {
      await ipc.librarySetToken(token().trim());
      setToken('');
      void refetch();
    } catch (err) {
      reportError('error.library', err);
    }
  };
  const remove = async () => {
    try {
      await ipc.libraryDeleteToken();
      void refetch();
    } catch (err) {
      reportError('error.library', err);
    }
  };
  return (
    <div class="af-question">
      <span class="af-question-label">{t('library.token')}</span>
      <p class="af-installer-hint">{t('library.tokenHint')}</p>
      <Show when={status()?.hasKey}>
        <p class="af-installer-hint">
          {t('library.tokenSaved', { last4: status()?.last4 ?? '••••' })}{' '}
          <button type="button" class="af-link-btn" onClick={() => void remove()}>
            {t('library.tokenRemove')}
          </button>
        </p>
      </Show>
      <div class="af-create-ai-row">
        <input
          class="af-input"
          type="password"
          autocomplete="off"
          aria-label={t('library.token')}
          placeholder="github_pat_…"
          value={token()}
          onInput={(e) => setToken(e.currentTarget.value)}
        />
        <button
          type="button"
          class="af-button secondary"
          disabled={!token().trim()}
          onClick={() => void save()}
        >
          {t('library.tokenSave')}
        </button>
      </div>
    </div>
  );
}

function CustomSources() {
  const [url, setUrl] = createSignal('');
  const [path, setPath] = createSignal('');
  const valid = () => GITHUB_URL.test(url().trim());
  const apply = async (mutate: (draft: ReactiveSettings) => void) => {
    updateSettings(mutate);
    // The backend reads the sources from settings: save before refreshing.
    await flushSettings();
    await refreshLibrary();
  };
  const add = () => {
    const entry = { url: url().trim(), path: path().trim() };
    setUrl('');
    setPath('');
    void apply((d) => {
      d.library.customSources.push(entry);
    });
  };
  return (
    <div class="af-question">
      <span class="af-question-label">{t('library.custom')}</span>
      <p class="af-installer-warning">{t('library.customWarning')}</p>
      <For each={settings.library.customSources}>
        {(source, index) => (
          <p class="af-lib-custom">
            <code>
              {source.url}
              {source.path ? ` › ${source.path}` : ''}
            </code>
            <button
              type="button"
              class="af-link-btn"
              onClick={() =>
                void apply((d) => {
                  d.library.customSources.splice(index(), 1);
                })
              }
            >
              {t('library.customRemove')}
            </button>
          </p>
        )}
      </For>
      <div class="af-create-ai-row">
        <input
          class="af-input"
          aria-label={t('library.customUrl')}
          placeholder="https://github.com/owner/repo"
          value={url()}
          onInput={(e) => setUrl(e.currentTarget.value)}
        />
        <input
          class="af-input"
          aria-label={t('library.customPath')}
          placeholder={t('library.customPath')}
          value={path()}
          onInput={(e) => setPath(e.currentTarget.value)}
        />
        <button type="button" class="af-button secondary" disabled={!valid()} onClick={add}>
          {t('library.customAdd')}
        </button>
      </div>
    </div>
  );
}

export function SourcesSection(props: { catalog: LibraryCatalog | null }) {
  const [open, setOpen] = createSignal<string | null>(null);
  const unsupported = (id: string) =>
    (props.catalog?.unsupported ?? []).filter((u) => u.sourceId === id);
  return (
    <div class="af-question-group af-lib-sources">
      <For each={props.catalog?.sources ?? []}>
        {(source) => (
          <div class="af-lib-source">
            <div class="af-plan-head">
              <strong>{source.name}</strong>
              <TrustBadge verified={source.verified} />
              <Show when={source.license}>
                <span class="af-badge">{source.license}</span>
              </Show>
              <span class="af-installer-hint">
                {t('library.sourceCounts', {
                  imported: source.imported,
                  unsupported: source.unsupported,
                })}
              </span>
              <Show when={source.unsupported > 0}>
                <button
                  type="button"
                  class="af-link-btn"
                  aria-expanded={open() === source.id}
                  onClick={() => setOpen((id) => (id === source.id ? null : source.id))}
                >
                  {t('library.showUnsupported')}
                </button>
              </Show>
            </div>
            <Show when={source.error}>
              {(error) => <p class="af-installer-warning">{errorText(error())}</p>}
            </Show>
            <Show when={open() === source.id}>
              <ul class="af-lib-unsupported">
                <For each={unsupported(source.id)}>
                  {(entry) => (
                    <li>
                      <code>{entry.path || source.repo}</code> — {reasonText(entry.reason)}
                      <Show when={entry.detail}>
                        <span class="af-installer-hint"> ({entry.detail})</span>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </div>
        )}
      </For>
      <CustomSources />
      <label class="af-check">
        <input
          type="checkbox"
          checked={settings.library.autoRefresh}
          onChange={(e) => {
            const on = e.currentTarget.checked;
            updateSettings((d) => {
              d.library.autoRefresh = on;
            });
          }}
        />
        {t('library.autoRefresh')}
      </label>
      <TokenField />
    </div>
  );
}
