// A file dragged from the tree onto a running terminal types its quoted relative path.
import { createSignal } from 'solid-js';
import { FILE_PATH_MIME } from '../../shared/dnd';
import { isWindows } from '../../shared/platform';
import { quotePath } from './quotePath';

export function createDropTarget(running: () => boolean, insert: (text: string) => void) {
  const [dropping, setDropping] = createSignal(false);
  const accepts = (event: DragEvent) =>
    running() && (event.dataTransfer?.types.includes(FILE_PATH_MIME) ?? false);
  return {
    dropping,
    onDragOver(event: DragEvent) {
      if (!accepts(event)) return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
      setDropping(true);
    },
    onDragLeave: () => setDropping(false),
    onDrop(event: DragEvent) {
      setDropping(false);
      const path = event.dataTransfer?.getData(FILE_PATH_MIME);
      if (!accepts(event) || !path) return;
      event.preventDefault();
      insert(quotePath(path, isWindows()));
    },
  };
}
