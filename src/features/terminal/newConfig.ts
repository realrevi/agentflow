import type { PtyKind } from '../../shared/types';
import { settings } from '../settings/settingsStore';
import { hasAgentUi, newHistoryKey, type TerminalConfig } from './config';

/**
 * A config for a new panel: starts in the current workspace folder (or home). The layout
 * assigns the character once the sprites are loaded.
 */
export function newTerminalConfig(kind: PtyKind, command: string | null = null): TerminalConfig {
  return {
    kind,
    command,
    args: [],
    cwd: settings.currentWorkspace,
    character: null,
    historyKey: hasAgentUi(kind) ? newHistoryKey() : null,
  };
}
