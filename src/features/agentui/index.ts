// Agent UI shared by every agent panel (terminal CLIs and API chats). Core feature: imports
// characters, i18n, settings and shared only. This entry is lightweight (state + types);
// the components (corner character, prompt bar) live in `./components`, which only the
// lazy panels import.
export type { CursorCell } from './overlay/follow';
export type { PromptHistory } from './prompt/history';
export { agentState, clearAgentState, setAgentState } from './state';
