// Public, lightweight API. TerminalPanel (and xterm.js) load lazily via the layout.
export {
  AGENT_KINDS,
  avatarFor,
  hasAgentUi,
  isAgentKind,
  kindLabel,
  parseTerminalConfig,
  terminalTitle,
  type AgentKind,
  wantsCharacter,
  type TerminalConfig,
} from './config';
export { agentState } from './agentActivity';
export { cliFor, cliStatus, loadCliStatus } from './cliStore';
export { newTerminalConfig } from './newConfig';
export { installNewShellShortcut } from './shortcuts';
export { clearTerminalStatus, setTerminalStatus, terminalStatus } from './statusStore';
export type { TerminalStatus } from './types';
