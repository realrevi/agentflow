// Step 4: every file the install writes, with its full path, what happens to it and a diff
// against what is there now. Conflicts (a file the installer did not create, or one edited
// since) need a tick each; the home folder needs its own tick. Install stays off until then.
import { For, Show, Suspense, type Component } from 'solid-js';
import type { InstallPlan, PlannedFile, PlanStatus } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import { openConflicts, TARGETS } from './answers';
import type { Wizard } from './wizardStore';
import { Button } from '../../../shared/ui/Button';
import { Checkbox } from '../../../shared/ui/Field';

export interface DiffProps {
  path: string;
  before: string | null;
  after: string;
}

const STATUS: Record<PlanStatus, I18nKey> = {
  create: 'installer.status.create',
  update: 'installer.status.update',
  unchanged: 'installer.status.unchanged',
  conflict: 'installer.status.conflict',
  remove: 'installer.status.remove',
};

export function PlanFile(props: {
  file: PlannedFile;
  Diff: Component<DiffProps>;
  confirmed: boolean;
  onConfirm: (on: boolean) => void;
}) {
  const target = () => TARGETS.find((info) => info.id === props.file.target);
  const label = () => (props.file.remove ? 'installer.status.remove' : STATUS[props.file.status]);
  return (
    <article class={`af-plan-file ${props.file.status}`}>
      <header class="af-plan-head">
        <code class="af-plan-path">{props.file.display}</code>
        <Show when={target()}>{(info) => <span class="af-badge">{t(info().label)}</span>}</Show>
        <Show when={props.file.kind === 'section'}>
          <span class="af-badge" title={t('installer.sectionHint')}>
            {t('installer.section')}
          </span>
        </Show>
        <span class={`af-badge status-${props.file.status}`}>{t(label())}</span>
      </header>
      <Show when={props.file.conflict}>
        {(kind) => (
          <label class="af-plan-conflict">
            <Checkbox
              checked={props.confirmed}
              onChange={(e) => props.onConfirm(e.currentTarget.checked)}
            />
            {t(kind() === 'foreign' ? 'installer.conflict.foreign' : 'installer.conflict.edited')}
          </label>
        )}
      </Show>
      <Show when={props.file.status !== 'unchanged'}>
        <div class="af-plan-diff">
          <Suspense fallback={<p class="af-installer-hint">{t('agent.permission.loading')}</p>}>
            <props.Diff
              path={props.file.path}
              before={props.file.before}
              after={props.file.after}
            />
          </Suspense>
        </div>
      </Show>
    </article>
  );
}

export function PlanList(props: {
  plan: InstallPlan;
  Diff: Component<DiffProps>;
  confirmed: (display: string) => boolean;
  onConfirm: (display: string, on: boolean) => void;
}) {
  return (
    <For each={props.plan.files}>
      {(file) => (
        <PlanFile
          file={file}
          Diff={props.Diff}
          confirmed={props.confirmed(file.display)}
          onConfirm={(on) => props.onConfirm(file.display, on)}
        />
      )}
    </For>
  );
}

export function StepPreview(props: { wizard: Wizard; Diff: Component<DiffProps> }) {
  const w = props.wizard;
  const blocked = () => {
    const plan = w.plan();
    if (!plan) return true;
    return openConflicts(plan, w.overwrite()).length > 0 || (plan.userScope && !w.userScopeOk());
  };
  return (
    <section class="af-wizard-step">
      <Show when={w.plan()}>
        {(plan) => (
          <>
            <p class="af-installer-hint">{t('installer.previewHint')}</p>
            <PlanList
              plan={plan()}
              Diff={props.Diff}
              confirmed={(display) => w.overwrite().has(display)}
              onConfirm={w.confirmOverwrite}
            />
            <Show when={plan().userScope}>
              <label class="af-plan-conflict">
                <Checkbox
                  checked={w.userScopeOk()}
                  onChange={(e) => w.setUserScopeOk(e.currentTarget.checked)}
                />
                {t('installer.userScopeConfirm')}
              </label>
            </Show>
          </>
        )}
      </Show>
      <div class="af-wizard-actions">
        <Button variant="secondary" onClick={() => w.setStep('questions')}>
          {t('installer.back')}
        </Button>
        <Button disabled={blocked() || w.busy()} onClick={() => void w.install()}>
          {t('installer.install')}
        </Button>
      </div>
    </section>
  );
}
