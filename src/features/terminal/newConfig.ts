import type { PtyKind } from '../../shared/types';
import { settings } from '../settings/settingsStore';
import type { TerminalConfig } from './config';

/** A config for a new panel: starts in the current workspace folder (or home). */
export function newTerminalConfig(kind: PtyKind, command: string | null = null): TerminalConfig {
  return { kind, command, args: [], cwd: settings.currentWorkspace };
}
