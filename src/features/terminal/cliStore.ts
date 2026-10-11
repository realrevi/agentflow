// Which agent CLIs are on PATH. The backend caches; `refresh` asks it to look again.
import { createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { CliStatus, PtyKind } from '../../shared/types';
import { reportError } from '../i18n';

const [cliStatus, setCliStatus] = createSignal<readonly CliStatus[] | null>(null);
let loading: Promise<void> | undefined;

export { cliStatus };

export function loadCliStatus(refresh = false): Promise<void> {
  if (loading) return loading;
  if (cliStatus() && !refresh) return Promise.resolve();
  loading = ipc
    .cliDetect(refresh)
    .then((status) => {
      setCliStatus(status);
    })
    .catch((err) => reportError('error.cliDetect', err))
    .finally(() => {
      loading = undefined;
    });
  return loading;
}

/** `undefined` until detection has run. */
export function cliFor(kind: PtyKind): CliStatus | undefined {
  return cliStatus()?.find((status) => status.kind === kind);
}
