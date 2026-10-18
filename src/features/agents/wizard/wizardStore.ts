// One installer wizard: template -> targets and scope -> questions -> preview -> done.
// Nothing is written before `install()`, and the backend re-plans then, so a stale preview
// can never be applied. Detection runs once per workspace (local, read-only).
import { batch, createSignal } from 'solid-js';
import { createStore, reconcile } from 'solid-js/store';
import { ipc } from '../../../shared/ipc';
import type {
  AgentTemplate,
  AnswerValue,
  Answers,
  Detection,
  InstallPlan,
  InstallResult,
  InstallScope,
  InstallTarget,
} from '../../../shared/types';
import { reportError } from '../../i18n';
import { prefill, supportsTarget } from './answers';

export type Step = 'template' | 'targets' | 'questions' | 'preview' | 'done';

export interface StartOptions {
  answers?: Answers;
  targets?: InstallTarget[];
  scope?: InstallScope;
}

export function createWizard(workspace: () => string | null, onInstalled: () => void) {
  const [step, setStep] = createSignal<Step>('template');
  const [template, setTemplate] = createSignal<AgentTemplate | null>(null);
  const [targets, setTargets] = createSignal<ReadonlySet<InstallTarget>>(new Set());
  const [scope, setScope] = createSignal<InstallScope>('project');
  const [answers, setAnswers] = createStore<Answers>({});
  const [detected, setDetected] = createSignal<ReadonlySet<string>>(new Set());
  const [detection, setDetection] = createSignal<Detection | null>(null);
  const [plan, setPlan] = createSignal<InstallPlan | null>(null);
  const [overwrite, setOverwrite] = createSignal<ReadonlySet<string>>(new Set());
  const [userScopeOk, setUserScopeOk] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [result, setResult] = createSignal<InstallResult | null>(null);
  let detectedFor: string | null = null;

  async function detect(): Promise<Detection | null> {
    const root = workspace();
    if (root === null) return null;
    if (detectedFor === root && detection()) return detection();
    try {
      const found = await ipc.agentsDetect(root);
      detectedFor = root;
      setDetection(found);
      return found;
    } catch (err) {
      reportError('error.installer', err);
      return null;
    }
  }

  async function start(next: AgentTemplate, options: StartOptions = {}): Promise<void> {
    const found = await detect();
    const filled = prefill(next, found, options.answers);
    const defaults: InstallTarget[] = ['claudeCode', 'agentflow'];
    const wanted = (options.targets ?? defaults).filter((id) => supportsTarget(next, id));
    batch(() => {
      setTemplate(next);
      setAnswers(reconcile(filled.answers));
      setDetected(filled.detected);
      setTargets(new Set(wanted));
      setScope(options.scope ?? 'project');
      setPlan(null);
      setResult(null);
      setStep('targets');
    });
  }

  function setAnswer(id: string, value: AnswerValue): void {
    setAnswers(id, value);
    // An edited answer is the user's, no longer the detector's.
    setDetected((ids) => new Set([...ids].filter((other) => other !== id)));
  }

  function toggleTarget(id: InstallTarget, on: boolean): void {
    setTargets((current) => {
      const next = new Set(current);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });
  }

  const request = () => {
    const current = template();
    return current
      ? { template: current, answers: { ...answers }, targets: [...targets()], scope: scope() }
      : null;
  };

  async function preview(): Promise<void> {
    const req = request();
    if (!req) return;
    setBusy(true);
    try {
      const planned = await ipc.agentsPlan(workspace(), req);
      batch(() => {
        setPlan(planned);
        setOverwrite(new Set<string>());
        setUserScopeOk(false);
        setStep('preview');
      });
    } catch (err) {
      reportError('error.installer', err);
    } finally {
      setBusy(false);
    }
  }

  function confirmOverwrite(display: string, on: boolean): void {
    setOverwrite((current) => {
      const next = new Set(current);
      if (on) next.add(display);
      else next.delete(display);
      return next;
    });
  }

  async function install(): Promise<void> {
    const req = request();
    if (!req) return;
    setBusy(true);
    try {
      const done = await ipc.agentsInstall(workspace(), req, {
        overwrite: [...overwrite()],
        userScope: userScopeOk(),
      });
      batch(() => {
        setResult(done);
        setStep('done');
      });
      onInstalled();
    } catch (err) {
      reportError('error.installer', err);
    } finally {
      setBusy(false);
    }
  }

  function reset(): void {
    batch(() => {
      setTemplate(null);
      setPlan(null);
      setResult(null);
      setStep('template');
    });
  }

  return {
    step,
    setStep,
    template,
    setTemplate,
    targets,
    toggleTarget,
    scope,
    setScope,
    answers,
    setAnswer,
    detected,
    detection,
    plan,
    overwrite,
    confirmOverwrite,
    userScopeOk,
    setUserScopeOk,
    busy,
    result,
    start,
    preview,
    install,
    reset,
  };
}

export type Wizard = ReturnType<typeof createWizard>;
