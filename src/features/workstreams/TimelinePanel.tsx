// Lazy-loaded dock panel: a workstream's handoffs from its log (default export), newest
// last, as clickable entries (the full text opens in place, as plain text). Filter by a
// panel; the Stage opens it filtered to one manager-worker pair.
import './workstreams.css';
import type { DockviewPanelApi } from 'dockview-core';
import { createResource, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { HandoffEntry } from '../../shared/types';
import { locale, reportError, t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { formatTokens } from '../tokens/format';
import { memberName, members } from './model';
import { onHandoffSent, workstreams } from './store';
import { filterEntries } from './timeline';

interface TimelineParams {
  workstream: string | null;
  pair: [string, string] | null;
}

const KINDS: Record<HandoffEntry['kind'], I18nKey> = {
  prompt: 'timeline.kind.prompt',
  report: 'timeline.kind.report',
  setup: 'timeline.kind.setup',
};

const MODES: Record<HandoffEntry['mode'], I18nKey> = {
  manual: 'workstream.mode.manual',
  semiAuto: 'workstream.mode.semiAuto',
  auto: 'workstream.mode.auto',
};

function Entry(props: { entry: HandoffEntry }) {
  const [open, setOpen] = createSignal(false);
  const time = () =>
    new Intl.DateTimeFormat(locale(), { timeStyle: 'medium' }).format(props.entry.at);
  const kb = () => (props.entry.size / 1024).toFixed(1);
  return (
    <li class="af-timeline-entry" classList={{ open: open() }}>
      <button type="button" class="af-timeline-head" onClick={() => setOpen(!open())}>
        <span class="af-timeline-time">{time()}</span>
        <span class="af-timeline-route">
          {props.entry.fromName} → {props.entry.toName}
        </span>
        <span class="af-badge">{t(KINDS[props.entry.kind])}</span>
        <span class="af-timeline-meta">
          {t(MODES[props.entry.mode])} · {t('timeline.size', { kb: kb() })}
          <Show when={props.entry.tokens !== null}>
            {' · '}
            {t('timeline.tokens', { tokens: formatTokens(props.entry.tokens ?? 0) })}
          </Show>
          <Show when={props.entry.truncated}> · {t('timeline.truncated')}</Show>
        </span>
      </button>
      <Show when={open()}>
        <pre class="af-timeline-text">{props.entry.text}</pre>
      </Show>
    </li>
  );
}

export default function TimelinePanel(props: { api: DockviewPanelApi; params: TimelineParams }) {
  const [params, setParams] = createSignal<TimelineParams>(props.params);
  const [panel, setPanel] = createSignal<string | null>(null);
  const selected = () => params().workstream ?? workstreams()[0]?.id ?? null;
  const ws = () => workstreams().find((item) => item.id === selected());
  const [entries, { refetch }] = createResource(selected, (id) =>
    ipc.workstreamLogRead(settings.currentWorkspace, id).catch((err: unknown) => {
      reportError('error.workstreamLogRead', err);
      return [] as HandoffEntry[];
    }),
  );
  onMount(() => {
    const subscription = props.api.onDidParametersChange((next) => {
      const raw = next as Partial<TimelineParams>;
      setParams({ workstream: raw.workstream ?? null, pair: raw.pair ?? null });
      setPanel(null);
    });
    // The log is appended after delivery; read it again shortly after a handoff.
    const stop = onHandoffSent((item) => {
      if (item.id === selected()) window.setTimeout(() => void refetch(), 300);
    });
    onCleanup(() => {
      subscription.dispose();
      stop();
    });
  });
  const shown = () => filterEntries(entries() ?? [], panel(), params().pair);

  return (
    <div class="af-timeline">
      <div class="af-timeline-bar">
        <select
          class="af-select"
          aria-label={t('timeline.workstream')}
          value={selected() ?? ''}
          onChange={(e) => setParams({ workstream: e.currentTarget.value, pair: null })}
        >
          <For each={workstreams()}>{(item) => <option value={item.id}>{item.name}</option>}</For>
        </select>
        <Show when={ws()}>
          {(current) => (
            <select
              class="af-select"
              aria-label={t('timeline.filter')}
              value={panel() ?? ''}
              onChange={(e) => {
                setPanel(e.currentTarget.value || null);
                setParams((p) => ({ ...p, pair: null }));
              }}
            >
              <option value="">{t('timeline.allPanels')}</option>
              <For each={members(current())}>
                {(id) => <option value={id}>{memberName(current(), id)}</option>}
              </For>
            </select>
          )}
        </Show>
        <Show when={params().pair && ws()}>
          <button
            type="button"
            class="af-link-btn"
            onClick={() => setParams((p) => ({ ...p, pair: null }))}
          >
            {t('timeline.clearPair')}
          </button>
        </Show>
      </div>
      <Show when={shown().length > 0} fallback={<p class="af-view-empty">{t('timeline.empty')}</p>}>
        <ul class="af-timeline-list">
          <For each={shown()}>{(entry) => <Entry entry={entry} />}</For>
        </ul>
      </Show>
    </div>
  );
}
