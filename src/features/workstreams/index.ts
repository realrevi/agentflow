// Manager–worker workstreams (phase 9): a manager agent hands focused tasks to its worker
// agents through the handoff protocol and reads their reports; modes manual, semi-auto and
// auto. Imports core features (agentui, characters, i18n, settings, tokens) and shared only;
// the layout and the app compose it.
export {
  closeNewWorkstream,
  newWorkstreamRequest,
  openNewWorkstream,
  type NewWorkstreamPreset,
} from './dialogState';
export { HandoffCards } from './HandoffCards';
export { memberName, members, WORKSTREAM_COLORS, type Handoff, type Workstream } from './model';
export { installWorkstreams, stopAll, type RuntimeHooks } from './runtime';
export { memberStatus, type MemberStatus } from './status';
export {
  anyAutoRunning,
  createWorkstream,
  dispatch,
  linkWorker,
  onHandoffSent,
  removeWorkstream,
  unlinkPanel,
  workstreamById,
  workstreamOf,
  workstreams,
} from './store';
export { WorkstreamsSection, type MemberView } from './WorkstreamsSection';
