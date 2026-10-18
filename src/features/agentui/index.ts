// Agent UI shared by every agent panel (terminal CLIs and API chats). Core feature: imports
// characters, i18n, settings and shared only. This entry is lightweight (state + types);
// the components (corner character, prompt bar) live in `./components`, which only the
// lazy panels import. Phase 9 adds what workstreams and the Stage read: activity, turn
// ends, attention, user input and the delivery ports.
export type { CursorCell } from './overlay/follow';
export type { PromptHistory } from './prompt/history';
export { agentState, clearAgentState, flashAgentState, setAgentState } from './state';
export {
  activityOf,
  clearActivity,
  kindText,
  setActivity,
  toolText,
  type Activity,
  type ActivityKind,
} from './activity';
export {
  attentionOf,
  clearAttention,
  glowing,
  installAttentionStyle,
  interactionClears,
  requestAttention,
  type AttentionReason,
} from './attention';
export { noteUserInput, onUserInput } from './input';
export { extraSystemOf, portOf, registerPort, setExtraSystem, type AgentPort } from './ports';
export {
  cliStarted,
  idleTurnEnded,
  logMatch,
  logToolPending,
  onTurnEnded,
  reportTurnEnded,
  startTurnFeed,
  type TurnEnd,
  type TurnSource,
} from './turns';
