import { describe, expect, it } from 'vitest';
import { back, isLast, next, savedStep, skip, startFlow, STEPS, stepId } from './flow';

describe('first-run flow', () => {
  it('starts at the welcome step, or resumes where the user left', () => {
    expect(stepId(startFlow(null))).toBe('welcome');
    expect(stepId(startFlow('theme'))).toBe('theme');
    expect(stepId(startFlow('nonsense'))).toBe('welcome');
  });

  it('goes forward and back within the steps', () => {
    let state = startFlow(null);
    state = next(next(state));
    expect(stepId(state)).toBe('theme');
    state = back(state);
    expect(stepId(state)).toBe('language');
    expect(stepId(back(back(back(state))))).toBe('welcome');
  });

  it('finishes after the last step and stops saving a resume point', () => {
    let state = startFlow('workspace');
    expect(isLast(state)).toBe(true);
    expect(savedStep(state)).toBe('workspace');
    state = next(state);
    expect(state.closed).toBe(true);
    expect(state.skipped).toBe(false);
    expect(savedStep(state)).toBeNull();
    expect(next(state)).toBe(state);
  });

  it('can be skipped from any step', () => {
    const state = skip(startFlow('character'));
    expect(state.closed).toBe(true);
    expect(state.skipped).toBe(true);
    expect(savedStep(state)).toBeNull();
  });

  it('walks every step once', () => {
    const seen: string[] = [];
    let state = startFlow(null);
    while (!state.closed) {
      seen.push(stepId(state));
      state = next(state);
    }
    expect(seen).toEqual([...STEPS]);
  });
});
