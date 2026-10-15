// Per-provider caches: the model list (fetched once per session, refreshable) and the key
// status (has key + last 4). Keys themselves never reach the frontend.
import { createStore } from 'solid-js/store';
import { ipc } from '../../shared/ipc';
import { toAppError } from '../../shared/errors';
import type { KeyStatus, ModelInfo } from '../../shared/types';

interface ModelsState {
  list: ModelInfo[];
  loading: boolean;
  error: string | null;
}

const [models, setModels] = createStore<Record<string, ModelsState>>({});
const [keys, setKeys] = createStore<Record<string, KeyStatus>>({});
const pending = new Map<string, Promise<ModelInfo[]>>();

/** Reactive. Empty until `loadModels` ran for this provider. */
export function modelsOf(providerId: string): ModelsState {
  return models[providerId] ?? { list: [], loading: false, error: null };
}

/** Fetches the provider's models (once, unless `refresh`). Errors stay in `modelsOf().error`. */
export function loadModels(providerId: string, refresh = false): Promise<ModelInfo[]> {
  const known = models[providerId];
  if (known && !refresh && !known.error && !known.loading) return Promise.resolve(known.list);
  const running = pending.get(providerId);
  if (running) return running;
  setModels(providerId, { list: known?.list ?? [], loading: true, error: null });
  const request = ipc
    .providerModels(providerId)
    .then((list) => {
      setModels(providerId, { list, loading: false, error: null });
      return list;
    })
    .catch((err: unknown) => {
      setModels(providerId, { list: [], loading: false, error: toAppError(err).message });
      return [];
    })
    .finally(() => pending.delete(providerId));
  pending.set(providerId, request);
  return request;
}

/** Forget a provider's models (its URL or key changed). */
export function forgetModels(providerId: string): void {
  setModels(providerId, undefined as unknown as ModelsState);
}

/** Reactive; `undefined` until `refreshKeyStatus` ran. */
export function keyStatusOf(providerId: string): KeyStatus | undefined {
  return keys[providerId];
}

export async function refreshKeyStatus(providerId: string): Promise<KeyStatus> {
  const status = await ipc.providerKeyStatus(providerId);
  setKeys(providerId, status);
  return status;
}

export async function saveKey(providerId: string, key: string): Promise<KeyStatus> {
  const status = await ipc.providerSetKey(providerId, key);
  setKeys(providerId, status);
  forgetModels(providerId);
  return status;
}

export async function removeKey(providerId: string): Promise<void> {
  await ipc.providerDeleteKey(providerId);
  setKeys(providerId, { hasKey: false, last4: null });
  forgetModels(providerId);
}
