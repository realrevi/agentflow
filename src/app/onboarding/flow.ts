// The first-run flow as a small pure state machine: six steps, back and next, skip and
// finish, and resuming at the step the user left (a reload or restart mid-flow).

export const STEPS = ['welcome', 'language', 'theme', 'character', 'connect', 'workspace'] as const;
export type StepId = (typeof STEPS)[number];

export interface FlowState {
  index: number;
  /** Finished (all steps) or skipped. */
  closed: boolean;
  skipped: boolean;
}

export const STORAGE_KEY = 'agentflow.onboarding.step';

/** Where to start: the saved step (resume) or the first one. */
export function startFlow(saved: string | null): FlowState {
  const index = STEPS.indexOf(saved as StepId);
  return { index: index < 0 ? 0 : index, closed: false, skipped: false };
}

export function stepId(state: FlowState): StepId {
  return STEPS[state.index] ?? 'welcome';
}

export function isLast(state: FlowState): boolean {
  return state.index >= STEPS.length - 1;
}

export function next(state: FlowState): FlowState {
  if (state.closed) return state;
  return isLast(state) ? { ...state, closed: true } : { ...state, index: state.index + 1 };
}

export function back(state: FlowState): FlowState {
  return state.closed ? state : { ...state, index: Math.max(0, state.index - 1) };
}

export function skip(state: FlowState): FlowState {
  return { ...state, closed: true, skipped: true };
}

/** The step to save for resuming, or null once the flow is closed (clear the saved step). */
export function savedStep(state: FlowState): StepId | null {
  return state.closed ? null : stepId(state);
}
