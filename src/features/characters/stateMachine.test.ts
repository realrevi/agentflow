import { describe, expect, it } from 'vitest';
import { frameAt, playbackOf } from './playback';
import { AgentStateTracker, ECHO_MS, QUIET_MS, REST_MS } from './stateMachine';

describe('agent state heuristic', () => {
  it('prompt -> think -> work -> done -> idle', () => {
    const agent = new AgentStateTracker();
    expect(agent.state).toBe('idle');
    expect(agent.prompt(1000)).toBe('think');
    expect(agent.output(1000 + ECHO_MS - 1)).toBe('think'); // the echo of the prompt
    expect(agent.output(1000 + ECHO_MS + 100)).toBe('work');
    const lastOutput = 1000 + ECHO_MS + 100;
    expect(agent.nextDeadline()).toBe(lastOutput + QUIET_MS);
    expect(agent.tick(lastOutput + QUIET_MS - 1)).toBe('work');
    expect(agent.tick(lastOutput + QUIET_MS)).toBe('done');
    expect(agent.nextDeadline()).toBe(lastOutput + QUIET_MS + REST_MS);
    expect(agent.tick(lastOutput + QUIET_MS + REST_MS)).toBe('idle');
    expect(agent.nextDeadline()).toBeNull();
  });

  it('more output keeps it working; output while done or idle works again', () => {
    const agent = new AgentStateTracker();
    expect(agent.output(0)).toBe('work');
    agent.output(1500);
    expect(agent.tick(2500)).toBe('work');
    expect(agent.tick(3500)).toBe('done');
    expect(agent.output(4000)).toBe('work');
  });

  it('the echo of keys typed into the terminal is not work', () => {
    const agent = new AgentStateTracker();
    agent.typed(1000);
    expect(agent.output(1000 + ECHO_MS - 1)).toBe('idle');
    expect(agent.output(1000 + ECHO_MS)).toBe('work');
    agent.typed(1500); // typing while it works does not stop the work
    expect(agent.output(1600)).toBe('work');
  });

  it('thinking does not time out without output', () => {
    const agent = new AgentStateTracker();
    agent.prompt(0);
    expect(agent.tick(60_000)).toBe('think');
    expect(agent.nextDeadline()).toBeNull();
  });

  it('exit code 0 sleeps, others error, restart goes idle', () => {
    const agent = new AgentStateTracker();
    agent.output(0);
    expect(agent.exit(0)).toBe('sleep');
    expect(agent.output(100)).toBe('sleep');
    expect(agent.prompt(200)).toBe('sleep');
    expect(agent.start()).toBe('idle');
    expect(agent.exit(1)).toBe('error');
    expect(agent.exit(null)).toBe('error');
  });
});

describe('sprite playback', () => {
  it('loops, plays once, or holds a frame', () => {
    expect(frameAt(playbackOf('walk', true), 4, 450, 100)).toBe(0);
    expect(frameAt(playbackOf('walk', true), 4, 350, 100)).toBe(3);
    expect(frameAt(playbackOf('receive', true), 3, 250, 100)).toBe(2);
    expect(frameAt(playbackOf('receive', true), 3, 300, 100)).toBeNull();
    expect(frameAt(playbackOf('idle', true), 4, 999, 100)).toBe(0);
    expect(frameAt(playbackOf('sleep', true), 2, 0, 100)).toBe(1);
    expect(playbackOf('work', false)).toEqual({ kind: 'still', frame: 'first' });
    expect(playbackOf('receive', false)).toEqual({ kind: 'still', frame: 'last' });
    expect(playbackOf('idle', true, true)).toEqual({ kind: 'loop' });
    expect(playbackOf('idle', false, true)).toEqual({ kind: 'still', frame: 'first' });
  });
});
