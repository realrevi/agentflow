// Lucide icons for tool cards, permission cards and the agent timeline.
import type { IconName } from '../../../shared/ui/Icon';

const ICONS: Record<string, IconName> = {
  list_dir: 'folder',
  read_file: 'file-text',
  glob: 'sparkles',
  search: 'search',
  write_file: 'file-plus',
  edit_file: 'pencil',
  run_command: 'terminal',
};

export function toolIcon(name: string): IconName {
  return ICONS[name] ?? 'terminal';
}
