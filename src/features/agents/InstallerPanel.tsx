// Lazy-loaded "Agents library" panel (see layout/renderers.tsx): the install wizard
// (template -> targets -> questions -> preview -> done), the community library (Browse)
// that feeds it, and the installed agents. The layout lends it the editor's diff view and
// "open file".
import './installer.css';
import { createResource, createSignal, Match, Show, Switch, type Component } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { AgentTemplate, LibraryItem } from '../../shared/types';
import { t, type I18nKey } from '../i18n';
import { currentWorkspace, openFolder } from '../workspace';
import { InstalledList } from './InstalledList';
import { BrowseTab } from './library/BrowseTab';
import { refreshProfiles } from './profilesStore';
import { StepDone } from './wizard/StepDone';
import { StepPreview, type DiffProps } from './wizard/StepPreview';
import { StepQuestions } from './wizard/StepQuestions';
import { StepTargets } from './wizard/StepTargets';
import { StepTemplate } from './wizard/StepTemplate';
import { createWizard, type Step } from './wizard/wizardStore';
import { Tabs } from '../../shared/ui/Segmented';
import { EmptyState } from '../../shared/ui/Surface';
import { Button } from '../../shared/ui/Button';

export interface InstallerHooks {
  Diff: Component<DiffProps>;
  openFile: (path: string) => void;
}

const STEPS: readonly { id: Step; label: I18nKey }[] = [
  { id: 'template', label: 'installer.step.template' },
  { id: 'targets', label: 'installer.step.targets' },
  { id: 'questions', label: 'installer.step.questions' },
  { id: 'preview', label: 'installer.step.preview' },
  { id: 'done', label: 'installer.step.done' },
];

export default function InstallerPanel(props: { hooks: InstallerHooks }) {
  const [tab, setTab] = createSignal<'install' | 'browse' | 'installed'>('install');
  const [version, setVersion] = createSignal(0);
  const changed = () => {
    setVersion((v) => v + 1);
    void refreshProfiles();
  };
  const wizard = createWizard(currentWorkspace, changed);
  const [starters] = createResource(() => ipc.agentsStarters().catch(() => [] as AgentTemplate[]));
  const current = () => STEPS.findIndex((s) => s.id === wizard.step());
  const installFromLibrary = async (template: AgentTemplate, item: LibraryItem) => {
    await wizard.start(template, { targets: item.native });
    setTab('install');
  };

  return (
    <div class="af-installer">
      <header class="af-installer-head">
        <h2 class="af-installer-title">{t('installer.title')}</h2>
        <Tabs
          label={t('installer.title')}
          value={tab()}
          onChange={setTab}
          options={[
            { value: 'install', label: t('installer.tab.install') },
            { value: 'browse', label: t('installer.tab.browse') },
            { value: 'installed', label: t('installer.tab.installed') },
          ]}
        />
      </header>
      <Show
        when={currentWorkspace()}
        fallback={
          <EmptyState
            icon="folder-open"
            text={t('installer.noWorkspace')}
            action={
              <Button icon="folder-open" onClick={() => void openFolder()}>
                {t('workspace.open')}
              </Button>
            }
          />
        }
      >
        <Show when={tab() === 'install'}>
          <ol class="af-wizard-steps">
            {STEPS.map((step, index) => (
              <li classList={{ active: index === current(), done: index < current() }}>
                {t(step.label)}
              </li>
            ))}
          </ol>
          <Switch>
            <Match when={wizard.step() === 'template'}>
              <StepTemplate wizard={wizard} workspace={currentWorkspace()} />
            </Match>
            <Match when={wizard.step() === 'targets'}>
              <StepTargets wizard={wizard} />
            </Match>
            <Match when={wizard.step() === 'questions'}>
              <StepQuestions wizard={wizard} />
            </Match>
            <Match when={wizard.step() === 'preview'}>
              <StepPreview wizard={wizard} Diff={props.hooks.Diff} />
            </Match>
            <Match when={wizard.step() === 'done'}>
              <StepDone wizard={wizard} onOpenFile={props.hooks.openFile} />
            </Match>
          </Switch>
        </Show>
        <Show when={tab() === 'browse'}>
          <BrowseTab
            workspace={currentWorkspace()}
            onInstall={(template, item) => void installFromLibrary(template, item)}
          />
        </Show>
        <Show when={tab() === 'installed'}>
          <InstalledList
            workspace={currentWorkspace()}
            wizard={wizard}
            starters={starters() ?? []}
            Diff={props.hooks.Diff}
            version={version}
            onEdit={() => setTab('install')}
            onChanged={changed}
            onBrowse={() => setTab('browse')}
          />
        </Show>
      </Show>
    </div>
  );
}
