// Installed agents of this workspace: change answers or update to a newer starter (both go
// through the wizard's preview), or uninstall: a preview of what goes; files edited since
// install stay unless ticked.
import { createResource, createSignal, For, Show, type Component } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { AgentTemplate, InstallPlan, InstalledAgent } from '../../shared/types';
import { reportError, t } from '../i18n';
import { newer, TARGETS } from './wizard/answers';
import { PlanList, type DiffProps } from './wizard/StepPreview';
import type { Wizard } from './wizard/wizardStore';

export function InstalledList(props: {
  workspace: string | null;
  wizard: Wizard;
  starters: readonly AgentTemplate[];
  Diff: Component<DiffProps>;
  version: () => number;
  onEdit: () => void;
  onChanged: () => void;
}) {
  const [installed, { refetch }] = createResource(
    () => [props.workspace, props.version()] as const,
    ([workspace]) => ipc.agentsInstalled(workspace).catch(() => [] as InstalledAgent[]),
  );
  const [removing, setRemoving] = createSignal<{ id: string; plan: InstallPlan } | null>(null);
  const [removeAnyway, setRemoveAnyway] = createSignal<ReadonlySet<string>>(new Set());
  const [userOk, setUserOk] = createSignal(false);

  const edit = async (agent: InstalledAgent, template = agent.template) => {
    await props.wizard.start(template, {
      answers: agent.answers,
      targets: agent.targets,
      scope: agent.scope,
    });
    props.onEdit();
  };
  const update = (agent: InstalledAgent) =>
    props.starters.find((s) => s.id === agent.id && newer(s.version, agent.template.version));

  const askRemove = async (id: string) => {
    try {
      setRemoveAnyway(new Set<string>());
      setUserOk(false);
      setRemoving({ id, plan: await ipc.agentsUninstallPlan(props.workspace, id) });
    } catch (err) {
      reportError('error.installer', err);
    }
  };
  const remove = async () => {
    const current = removing();
    if (!current) return;
    const conflicts = current.plan.files.filter((f) => f.conflict !== null).map((f) => f.display);
    const keep = conflicts.filter((display) => !removeAnyway().has(display));
    try {
      await ipc.agentsUninstall(
        props.workspace,
        current.id,
        { overwrite: [...removeAnyway()], userScope: userOk() },
        keep,
      );
      setRemoving(null);
      showToast({ kind: 'success', title: t('installer.uninstalled') });
      void refetch();
      props.onChanged();
    } catch (err) {
      reportError('error.installer', err);
    }
  };

  return (
    <section class="af-wizard-step">
      <Show
        when={(installed() ?? []).length > 0}
        fallback={<p class="af-installer-hint">{t('installer.none')}</p>}
      >
        <For each={installed() ?? []}>
          {(agent) => (
            <article class="af-installed">
              <header class="af-plan-head">
                <strong>{agent.template.name}</strong>
                <span class="af-badge">v{agent.template.version}</span>
                <For each={TARGETS.filter((target) => agent.targets.includes(target.id))}>
                  {(target) => <span class="af-badge">{t(target.label)}</span>}
                </For>
                <Show when={agent.scope === 'user'}>
                  <span class="af-badge">{t('installer.scope.user')}</span>
                </Show>
              </header>
              <p class="af-installer-hint">
                {t('installer.updatedAt', { time: new Date(agent.updatedAt).toLocaleString() })}
              </p>
              <div class="af-wizard-actions start">
                <Show when={update(agent)}>
                  {(next) => (
                    <button
                      type="button"
                      class="af-button"
                      onClick={() => void edit(agent, next())}
                    >
                      {t('installer.update', { version: next().version })}
                    </button>
                  )}
                </Show>
                <button type="button" class="af-button secondary" onClick={() => void edit(agent)}>
                  {t('installer.changeAnswers')}
                </button>
                <button
                  type="button"
                  class="af-button danger"
                  onClick={() => void askRemove(agent.id)}
                >
                  {t('installer.uninstall')}
                </button>
              </div>
              <Show when={removing()?.id === agent.id && removing()}>
                {(current) => (
                  <div class="af-uninstall">
                    <p class="af-installer-hint">{t('installer.uninstallHint')}</p>
                    <PlanList
                      plan={current().plan}
                      Diff={props.Diff}
                      confirmed={(display) => removeAnyway().has(display)}
                      onConfirm={(display, on) =>
                        setRemoveAnyway((set) => {
                          const next = new Set(set);
                          if (on) next.add(display);
                          else next.delete(display);
                          return next;
                        })
                      }
                    />
                    <Show when={current().plan.userScope}>
                      <label class="af-plan-conflict">
                        <input
                          type="checkbox"
                          checked={userOk()}
                          onChange={(e) => setUserOk(e.currentTarget.checked)}
                        />
                        {t('installer.userScopeConfirm')}
                      </label>
                    </Show>
                    <div class="af-wizard-actions">
                      <button
                        type="button"
                        class="af-button secondary"
                        onClick={() => setRemoving(null)}
                      >
                        {t('dialog.cancel')}
                      </button>
                      <button
                        type="button"
                        class="af-button danger"
                        disabled={current().plan.userScope && !userOk()}
                        onClick={() => void remove()}
                      >
                        {t('installer.uninstall')}
                      </button>
                    </div>
                  </div>
                )}
              </Show>
            </article>
          )}
        </For>
      </Show>
    </section>
  );
}
