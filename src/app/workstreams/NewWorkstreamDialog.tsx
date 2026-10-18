// "New workstream": a name, a color, the handoff mode, one manager and 1..n workers, each
// an open agent panel (in no workstream yet) or a new one. New panels open first; members
// are named after their pixel character ("Fox"), unique within the workstream.
import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import type { HandoffMode } from '../../shared/types';
import { t, type I18nKey } from '../../features/i18n';
import { focusPanel, openPanels, openStage, panelCharacterName } from '../../features/layout';
import { settings } from '../../features/settings';
import {
  closeNewWorkstream,
  createWorkstream,
  newWorkstreamRequest,
  WORKSTREAM_COLORS,
  workstreams,
  type NewWorkstreamPreset,
} from '../../features/workstreams';
import { createMember, memberOptions, uniqueNames, type MemberOption } from './memberOptions';

const MODES: { mode: HandoffMode; key: I18nKey }[] = [
  { mode: 'manual', key: 'workstream.mode.manual' },
  { mode: 'semiAuto', key: 'workstream.mode.semiAuto' },
  { mode: 'auto', key: 'workstream.mode.auto' },
];

function MemberSelect(props: {
  value: string;
  options: readonly MemberOption[];
  label: string;
  onChange: (value: string) => void;
}) {
  const group = (which: MemberOption['group']) => props.options.filter((o) => o.group === which);
  return (
    <select
      class="af-select"
      aria-label={props.label}
      value={props.value}
      onChange={(event) => props.onChange(event.currentTarget.value)}
    >
      <Show when={group('open').length > 0}>
        <optgroup label={t('workstream.dialog.open')}>
          <For each={group('open')}>{(o) => <option value={o.value}>{o.label}</option>}</For>
        </optgroup>
      </Show>
      <optgroup label={t('workstream.dialog.newGroup')}>
        <For each={group('new')}>{(o) => <option value={o.value}>{o.label}</option>}</For>
      </optgroup>
    </select>
  );
}

function Form(props: { preset: NewWorkstreamPreset }) {
  const options = memberOptions();
  const pick = (id: string | undefined) =>
    id && options.some((o) => o.value === `panel:${id}`) ? `panel:${id}` : undefined;
  const count = workstreams().length;
  const [name, setName] = createSignal(t('workstream.dialog.defaultName', { n: count + 1 }));
  const [color, setColor] = createSignal<string>(
    WORKSTREAM_COLORS[count % WORKSTREAM_COLORS.length] ?? '#F5C542',
  );
  const [mode, setMode] = createSignal<HandoffMode>(settings.workstreams.defaultMode);
  const [manager, setManager] = createSignal(
    pick(props.preset.manager) ?? options[0]?.value ?? 'cli:claude',
  );
  const presetWorkers = (props.preset.workers ?? []).flatMap((id) => pick(id) ?? []);
  const [workers, setWorkers] = createSignal<string[]>(
    presetWorkers.length > 0 ? presetWorkers : ['cli:claude'],
  );
  const [busy, setBusy] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  const validate = (): string | null => {
    if (!name().trim()) return t('workstream.dialog.needName');
    if (workers().length === 0) return t('workstream.dialog.needWorker');
    const open = [manager(), ...workers()].filter((v) => v.startsWith('panel:'));
    if (new Set(open).size !== open.length) return t('workstream.dialog.duplicate');
    return null;
  };

  const create = async () => {
    const invalid = validate();
    setProblem(invalid);
    if (invalid || busy()) return;
    setBusy(true);
    try {
      const managerId = await createMember(manager());
      const workerIds: string[] = [];
      for (const value of workers()) {
        const id = await createMember(value);
        if (id) workerIds.push(id);
      }
      if (!managerId || workerIds.length === 0) {
        setProblem(t('workstream.dialog.failed'));
        return;
      }
      const titles = new Map(openPanels().map((panel) => [panel.id, panel.title]));
      const wanted: [string, string][] = [];
      for (const id of [managerId, ...workerIds]) {
        wanted.push([id, (await panelCharacterName(id)) ?? titles.get(id) ?? id]);
      }
      createWorkstream({
        name: name(),
        color: color(),
        mode: mode(),
        manager: managerId,
        workers: workerIds,
        names: uniqueNames(wanted),
      });
      openStage(true);
      focusPanel(managerId);
      closeNewWorkstream();
    } finally {
      setBusy(false);
    }
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return;
    event.stopPropagation();
    closeNewWorkstream();
  };
  onMount(() => document.addEventListener('keydown', onKeyDown, true));
  onCleanup(() => document.removeEventListener('keydown', onKeyDown, true));

  return (
    <div
      class="af-dialog-backdrop"
      onPointerDown={(event) => event.target === event.currentTarget && closeNewWorkstream()}
    >
      <form
        class="af-dialog af-ws-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="af-ws-dialog-title"
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <h2 id="af-ws-dialog-title" class="af-dialog-title">
          {t('workstream.dialog.title')}
        </h2>
        <label class="af-ws-field">
          <span>{t('workstream.dialog.name')}</span>
          <input class="af-input" value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        </label>
        <div class="af-ws-field">
          <span>{t('workstream.dialog.color')}</span>
          <div class="af-ws-swatches" role="radiogroup" aria-label={t('workstream.dialog.color')}>
            <For each={WORKSTREAM_COLORS}>
              {(swatch) => (
                <button
                  type="button"
                  role="radio"
                  aria-checked={color() === swatch}
                  aria-label={swatch}
                  class="af-ws-swatch"
                  classList={{ active: color() === swatch }}
                  style={{ background: swatch }}
                  onClick={() => setColor(swatch)}
                />
              )}
            </For>
          </div>
        </div>
        <div class="af-ws-field">
          <span>{t('workstream.mode')}</span>
          <div class="af-mode-pill" role="radiogroup" aria-label={t('workstream.mode')}>
            <For each={MODES}>
              {(entry) => (
                <button
                  type="button"
                  role="radio"
                  aria-checked={mode() === entry.mode}
                  classList={{ active: mode() === entry.mode }}
                  onClick={() => setMode(entry.mode)}
                >
                  {t(entry.key)}
                </button>
              )}
            </For>
          </div>
          <p class="af-ws-hint">{t(`workstream.dialog.mode.${mode()}`)}</p>
        </div>
        <div class="af-ws-field">
          <span>{t('workstream.dialog.manager')}</span>
          <MemberSelect
            value={manager()}
            options={options}
            label={t('workstream.dialog.manager')}
            onChange={setManager}
          />
        </div>
        <div class="af-ws-field">
          <span>{t('workstream.dialog.workers')}</span>
          <For each={workers()}>
            {(value, index) => (
              <div class="af-ws-worker-row">
                <MemberSelect
                  value={value}
                  options={options}
                  label={t('workstream.dialog.worker', { n: index() + 1 })}
                  onChange={(next) =>
                    setWorkers((list) => list.map((item, i) => (i === index() ? next : item)))
                  }
                />
                <button
                  type="button"
                  class="af-icon-btn"
                  aria-label={t('workstream.dialog.removeWorker')}
                  title={t('workstream.dialog.removeWorker')}
                  onClick={() => setWorkers((list) => list.filter((_, i) => i !== index()))}
                >
                  ×
                </button>
              </div>
            )}
          </For>
          <button
            type="button"
            class="af-link-btn"
            onClick={() => setWorkers((list) => [...list, 'cli:claude'])}
          >
            {t('workstream.dialog.addWorker')}
          </button>
        </div>
        <Show when={problem()}>{(text) => <p class="af-ws-problem">{text()}</p>}</Show>
        <div class="af-dialog-actions">
          <button type="button" class="af-button secondary" onClick={closeNewWorkstream}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" class="af-button" disabled={busy()}>
            {t('workstream.dialog.submit')}
          </button>
        </div>
      </form>
    </div>
  );
}

export function NewWorkstreamDialog() {
  return (
    <Show when={newWorkstreamRequest()} keyed>
      {(preset) => (
        <Portal>
          <Form preset={preset} />
        </Portal>
      )}
    </Show>
  );
}
