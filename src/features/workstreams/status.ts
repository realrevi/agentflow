// The status a workstream shows for each of its panels (sidebar, Stage tooltip).
import type { AgentState } from '../characters/stateMachine';
import type { Workstream } from './model';

export type MemberStatus = 'working' | 'waiting' | 'done' | 'error' | 'paused' | 'idle';

export function memberStatus(
  ws: Workstream,
  panel: string,
  state: AgentState,
  needsUser: boolean,
): MemberStatus {
  if (state === 'think' || state === 'work') return 'working';
  if (state === 'wait' || needsUser || ws.pending?.from === panel) return 'waiting';
  if (state === 'error') return 'error';
  if (ws.status === 'paused') return 'paused';
  if (state === 'done' || ws.status === 'done') return 'done';
  return 'idle';
}
