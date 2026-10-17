// Why a command always asks: the reason ids of src-tauri/src/agent/dangerous.rs, localized.
import { t, type I18nKey } from '../../i18n';

export const DANGER_REASONS: Readonly<Record<string, I18nKey>> = {
  recursiveDelete: 'agent.danger.recursiveDelete',
  rootDelete: 'agent.danger.rootDelete',
  formatDrive: 'agent.danger.formatDrive',
  diskTool: 'agent.danger.diskTool',
  rawDisk: 'agent.danger.rawDisk',
  forcePush: 'agent.danger.forcePush',
  hardReset: 'agent.danger.hardReset',
  gitClean: 'agent.danger.gitClean',
  pipeToShell: 'agent.danger.pipeToShell',
  runDownload: 'agent.danger.runDownload',
  shutdown: 'agent.danger.shutdown',
  forkBomb: 'agent.danger.forkBomb',
};

export function dangerText(id: string): string {
  const key = DANGER_REASONS[id];
  return key ? t(key) : id;
}
