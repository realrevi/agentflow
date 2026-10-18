// The first-run flow (lazy: loaded only while it is not done): welcome, language, theme
// with live preview, character, connect an AI, workspace. Skippable at every step and
// resumable after a restart; Settings > Appearance can open it again.
import './onboarding.css';
import { createSignal, Match, Switch } from 'solid-js';
import type { CharacterRef } from '../../features/characters';
import { t, type I18nKey } from '../../features/i18n';
import { updateSettings } from '../../features/settings';
import { Button } from '../../shared/ui/Button';
import { Dialog } from '../../shared/ui/Dialog';
import { openReadyAgent } from './finish';
import {
  back,
  next,
  savedStep,
  skip,
  startFlow,
  STEPS,
  STORAGE_KEY,
  stepId,
  type FlowState,
  type StepId,
} from './flow';
import {
  CharacterStep,
  ConnectStep,
  LanguageStep,
  ThemeStep,
  WelcomeStep,
  WorkspaceStep,
} from './Steps';

const TITLES: Record<StepId, I18nKey> = {
  welcome: 'onboarding.welcome',
  language: 'onboarding.language',
  theme: 'onboarding.theme',
  character: 'onboarding.character',
  connect: 'onboarding.connect',
  workspace: 'onboarding.workspace',
};

function readSaved(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function writeSaved(step: string | null): void {
  try {
    if (step) localStorage.setItem(STORAGE_KEY, step);
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Storage unavailable: the flow just starts at the beginning next time.
  }
}

export default function Onboarding() {
  const [state, setState] = createSignal<FlowState>(startFlow(readSaved()));
  const [character, setCharacter] = createSignal<CharacterRef | null>(null);
  const [open, setOpen] = createSignal(true);

  const move = (to: FlowState) => {
    setState(to);
    writeSaved(savedStep(to));
    if (!to.closed) return;
    setOpen(false);
    updateSettings((s) => (s.onboarding.done = true));
    if (!to.skipped) void openReadyAgent(character());
  };

  const step = () => stepId(state());
  const index = () => state().index;

  return (
    <Dialog
      open={open()}
      title={t(TITLES[step()])}
      class="af-onboarding"
      dismissable={false}
      initialFocus=".af-onboarding-next"
      onClose={() => move(skip(state()))}
      actions={
        <>
          <Button variant="ghost" onClick={() => move(skip(state()))}>
            {t('onboarding.skip')}
          </Button>
          <span class="af-onboarding-spacer" />
          <Button variant="secondary" disabled={index() === 0} onClick={() => move(back(state()))}>
            {t('onboarding.back')}
          </Button>
          <Button class="af-onboarding-next" onClick={() => move(next(state()))}>
            {t(index() === STEPS.length - 1 ? 'onboarding.finish' : 'onboarding.next')}
          </Button>
        </>
      }
    >
      <ol
        class="af-onboarding-dots"
        aria-label={t('onboarding.progress', { n: index() + 1, total: STEPS.length })}
      >
        {STEPS.map((id, i) => (
          <li
            classList={{ active: i === index(), done: i < index() }}
            aria-current={i === index() ? 'step' : undefined}
          >
            <span class="af-sr-only">{t(TITLES[id])}</span>
          </li>
        ))}
      </ol>
      <div class="af-onboarding-body">
        <Switch>
          <Match when={step() === 'welcome'}>
            <WelcomeStep />
          </Match>
          <Match when={step() === 'language'}>
            <LanguageStep />
          </Match>
          <Match when={step() === 'theme'}>
            <ThemeStep />
          </Match>
          <Match when={step() === 'character'}>
            <CharacterStep character={character()} onPick={setCharacter} />
          </Match>
          <Match when={step() === 'connect'}>
            <ConnectStep />
          </Match>
          <Match when={step() === 'workspace'}>
            <WorkspaceStep />
          </Match>
        </Switch>
      </div>
    </Dialog>
  );
}
