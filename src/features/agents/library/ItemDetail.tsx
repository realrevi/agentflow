// One library item in full: the original text rendered (sanitized), its files, source link,
// author, license, last commit and every safety flag. "Install" and "Customize with AI"
// hand its template to the wizard, whose preview shows every file before anything is
// written. A skill's other files download on first view (then they are cached).
import { createMemo, createResource, createSignal, For, Show } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import { renderMarkdown } from '../../../shared/markdown';
import { webUrl } from '../../../shared/url';
import type { AgentTemplate, LibraryDetail, LibraryItem } from '../../../shared/types';
import { reportError, t } from '../../i18n';
import { formatTokens } from '../../tokens';
import { CustomizeAi } from './CustomizeAi';
import { TrustBadge } from './ItemCard';
import { FLAG_LABELS, FORMAT_LABELS, KIND_LABELS } from './labels';
import { Button } from '../../../shared/ui/Button';

const FRONTMATTER = /^---\r?\n[\s\S]*?\r?\n---\r?\n?/;

/** The text as the reader sees it: Markdown without its frontmatter; TOML as code. */
function display(detail: LibraryDetail): { head: string; body: string } {
  if (detail.item.path.endsWith('.toml'))
    return { head: '', body: '```toml\n' + detail.text + '\n```' };
  const head = FRONTMATTER.exec(detail.text)?.[0] ?? '';
  return { head: head.trim(), body: detail.text.slice(head.length) };
}

function size(bytes: number): string {
  return bytes < 1024 ? `${bytes} B` : `${Math.round(bytes / 1024)} KB`;
}

export function ItemDetail(props: {
  item: LibraryItem;
  workspace: string | null;
  onBack: () => void;
  onInstall: (template: AgentTemplate, item: LibraryItem) => void;
}) {
  const [detail] = createResource(
    () => props.item.id,
    (id) =>
      ipc.libraryDetail(id).catch((err: unknown) => {
        reportError('error.library', err);
        return null;
      }),
  );
  const [customizing, setCustomizing] = createSignal(false);
  const text = createMemo(() => {
    const current = detail();
    return current ? display(current) : null;
  });
  const html = createMemo(() => {
    const current = text();
    return current ? renderMarkdown(current.body) : '';
  });
  const item = () => detail()?.item ?? props.item;
  const openUrl = (url: string) =>
    void ipc.openUrl(url).catch((err: unknown) => reportError('error.openLink', err));
  const open = () => openUrl(item().url);
  // Links in the item open in the browser (relative ones are files of the item: no-op).
  const onLinkClick = (event: MouseEvent) => {
    const link = (event.target as HTMLElement).closest<HTMLAnchorElement>('a[href]');
    if (!link) return;
    event.preventDefault();
    const url = webUrl(link.getAttribute('href') ?? '');
    if (url) openUrl(url);
  };

  return (
    <section class="af-wizard-step af-lib-detail">
      <div class="af-wizard-actions start">
        <Button variant="secondary" onClick={() => props.onBack()}>
          {t('installer.back')}
        </Button>
      </div>
      <header class="af-plan-head">
        <h3 class="af-installer-subtitle">{item().name}</h3>
        <span class="af-badge">{t(KIND_LABELS[item().kind])}</span>
        <TrustBadge verified={item().verified} />
      </header>
      <p class="af-lib-card-desc">{item().description}</p>
      <Show when={!item().verified}>
        <p class="af-installer-warning">{t('library.unverifiedWarning')}</p>
      </Show>
      <dl class="af-lib-meta">
        <dt>{t('library.source')}</dt>
        <dd>
          <Button variant="ghost" size="sm" onClick={open}>
            {item().repo}/{item().path}
          </Button>
        </dd>
        <dt>{t('library.author')}</dt>
        <dd>{item().author}</dd>
        <dt>{t('library.license')}</dt>
        <dd>{item().license}</dd>
        <dt>{t('library.updated')}</dt>
        <dd>
          {item().updatedAt ? new Date(item().updatedAt ?? '').toLocaleDateString() : '—'} ·{' '}
          <code>{item().commit.slice(0, 7)}</code>
        </dd>
        <dt>{t('library.format')}</dt>
        <dd>{t(FORMAT_LABELS[item().format])}</dd>
        <dt>{t('library.size')}</dt>
        <dd>{t('library.tokens', { count: formatTokens(item().tokens) })}</dd>
      </dl>
      <Show when={detail.loading}>
        <p class="af-installer-hint">{t('library.loading')}</p>
      </Show>
      <Show when={detail()}>
        {(current) => (
          <>
            <Show when={current().flags.length > 0}>
              <div class="af-lib-flags">
                <strong>{t('library.flags')}</strong>
                <p class="af-installer-hint">{t('library.flagsHint')}</p>
                <ul>
                  <For each={current().flags}>
                    {(flag) => (
                      <li>
                        <span class="af-lib-flag">⚠ {t(FLAG_LABELS[flag.id])}</span>
                        <Show when={flag.file}>
                          <code class="af-lib-flag-file">{flag.file}</code>
                        </Show>
                        <code class="af-lib-excerpt">{flag.excerpt}</code>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </Show>
            <Show when={current().files.length > 0}>
              <div class="af-lib-files">
                <strong>{t('library.files')}</strong>
                <ul>
                  <For each={current().files}>
                    {(file) => (
                      <li>
                        <code>{file.path}</code>{' '}
                        <span class="af-installer-hint">{size(file.size)}</span>
                        <Show when={file.executable}>
                          <span class="af-lib-flag">{t('library.script')}</span>
                        </Show>
                        <Show when={file.binary}>
                          <span class="af-badge">{t('library.binary')}</span>
                        </Show>
                        <Show when={!file.cached}>
                          <span class="af-badge">{t('library.notCached')}</span>
                        </Show>
                      </li>
                    )}
                  </For>
                </ul>
                <p class="af-installer-hint">{t('library.scriptsHint')}</p>
              </div>
            </Show>
            <Show when={current().problem}>
              {(problem) => (
                <p class="af-installer-warning">
                  {t(
                    problem() === 'notCached'
                      ? 'library.problem.notCached'
                      : 'library.problem.invalid',
                  )}
                </p>
              )}
            </Show>
            <Show when={current().template}>
              {(template) => (
                <>
                  <div class="af-wizard-actions start">
                    <Button onClick={() => props.onInstall(template(), item())}>
                      {t('library.install')}
                    </Button>
                    <Button
                      variant="secondary"
                      aria-expanded={customizing()}
                      onClick={() => setCustomizing((on) => !on)}
                    >
                      {t('library.customize')}
                    </Button>
                  </div>
                  <Show when={customizing()}>
                    <CustomizeAi
                      workspace={props.workspace}
                      template={template()}
                      onDraft={(draft) => props.onInstall(draft, item())}
                    />
                  </Show>
                </>
              )}
            </Show>
            <Show when={text()?.head}>
              <details class="af-lib-frontmatter">
                <summary>{t('library.frontmatter')}</summary>
                <pre>{text()?.head}</pre>
              </details>
            </Show>
            {/* Sanitized by renderMarkdown (raw HTML is shown as text, DOMPurify after). */}
            <div class="af-lib-markdown af-md" innerHTML={html()} onClick={onLinkClick} />
          </>
        )}
      </Show>
    </section>
  );
}
