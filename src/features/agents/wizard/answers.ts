// The wizard's answers: prefilled from an earlier install, then the workspace detectors,
// then the template defaults; detected values are marked so the UI can say so. Plus the
// question groups, the required check and what each target supports.
import type {
  AgentTemplate,
  AnswerValue,
  Answers,
  Detection,
  InstallPlan,
  InstallTarget,
  Question,
} from '../../../shared/types';
import type { I18nKey } from '../../i18n';

const TEXT_KINDS = new Set(['text', 'multiline', 'color', 'font', 'choice']);

function empty(question: Question): AnswerValue {
  if (question.type === 'boolean') return false;
  if (question.type === 'multiChoice') return [];
  return '';
}

export function isEmpty(value: AnswerValue | undefined): boolean {
  if (value === undefined) return true;
  if (typeof value === 'boolean') return false;
  return typeof value === 'string' ? value.trim() === '' : value.length === 0;
}

export interface Prefill {
  answers: Answers;
  /** Question ids whose answer came from a detector. */
  detected: ReadonlySet<string>;
}

export function prefill(
  template: AgentTemplate,
  detection: Detection | null,
  previous: Answers = {},
): Prefill {
  const answers: Answers = {};
  const detected = new Set<string>();
  for (const question of template.questions ?? []) {
    const earlier = previous[question.id];
    if (earlier !== undefined) {
      answers[question.id] = earlier;
      continue;
    }
    const found = question.detect ? detection?.values[question.detect] : undefined;
    // A detected value only fills free-text answers (choices must stay one of the options).
    const usable =
      found !== undefined &&
      found.trim() !== '' &&
      TEXT_KINDS.has(question.type) &&
      (question.type !== 'choice' || (question.options ?? []).includes(found));
    if (usable) {
      answers[question.id] = found;
      detected.add(question.id);
    } else {
      answers[question.id] = question.default ?? empty(question);
    }
  }
  return { answers, detected };
}

/** Questions by group, in the order groups first appear. */
export function groups(questions: readonly Question[]): { name: string; questions: Question[] }[] {
  const out: { name: string; questions: Question[] }[] = [];
  for (const question of questions) {
    const name = question.group ?? '';
    const group = out.find((g) => g.name === name);
    if (group) group.questions.push(question);
    else out.push({ name, questions: [question] });
  }
  return out;
}

/** Labels of required questions that still have no answer. */
export function missing(template: AgentTemplate, answers: Answers): string[] {
  return (template.questions ?? [])
    .filter((q) => q.required && q.type !== 'boolean' && isEmpty(answers[q.id]))
    .map((q) => q.label);
}

export interface TargetInfo {
  id: InstallTarget;
  label: I18nKey;
  /** The tool documents a home-folder location. */
  userScope: boolean;
  hint: I18nKey;
}

export const TARGETS: readonly TargetInfo[] = [
  {
    id: 'claudeCode',
    label: 'installer.target.claudeCode',
    userScope: true,
    hint: 'installer.hint.claudeCode',
  },
  { id: 'codex', label: 'installer.target.codex', userScope: true, hint: 'installer.hint.codex' },
  {
    id: 'gemini',
    label: 'installer.target.gemini',
    userScope: true,
    hint: 'installer.hint.gemini',
  },
  {
    id: 'cursor',
    label: 'installer.target.cursor',
    userScope: false,
    hint: 'installer.hint.cursor',
  },
  {
    id: 'agentflow',
    label: 'installer.target.agentflow',
    userScope: false,
    hint: 'installer.hint.agentflow',
  },
];

/** Conflicts the user has not confirmed yet (by `display` path). */
export function openConflicts(plan: InstallPlan, overwrite: ReadonlySet<string>): string[] {
  return plan.files
    .filter((f) => f.conflict !== null && !overwrite.has(f.display))
    .map((f) => f.display);
}

/** A newer starter version than the installed one (`major.minor.patch`). */
export function newer(available: string, installed: string): boolean {
  const parts = (v: string) => v.split('.').map((n) => Number.parseInt(n, 10) || 0);
  const [a, b] = [parts(available), parts(installed)];
  for (let i = 0; i < 3; i += 1) {
    if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  }
  return false;
}

/** Whether a template installs for a tool (mirrors `targets::supports` in Rust): skills
 * where the tool has skill folders, Gemini commands only for Gemini CLI. */
export function supportsTarget(template: AgentTemplate, target: InstallTarget): boolean {
  switch (template.kind ?? 'agent') {
    case 'skill':
      return target !== 'agentflow';
    case 'command':
      return target === 'gemini';
    default:
      return true;
  }
}

/** How to use the installed template in a tool (skills and commands differ from agents). */
export function hintFor(template: AgentTemplate, target: TargetInfo): I18nKey {
  if (template.kind === 'skill') return 'installer.hint.skill';
  if (template.kind === 'command') return 'installer.hint.command';
  return target.hint;
}
