import { createSignal } from 'solid-js';

export type ToastKind = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  detail?: string;
}

const AUTO_DISMISS_MS: Record<ToastKind, number> = { info: 3500, success: 3500, error: 7000 };
const MAX_TOASTS = 4;

const [toasts, setToasts] = createSignal<readonly Toast[]>([]);
let nextId = 1;

export { toasts };

export function showToast(toast: Omit<Toast, 'id'>): void {
  const id = nextId++;
  setToasts((list) => [...list, { ...toast, id }].slice(-MAX_TOASTS));
  window.setTimeout(() => dismissToast(id), AUTO_DISMISS_MS[toast.kind]);
}

export function dismissToast(id: number): void {
  setToasts((list) => list.filter((toast) => toast.id !== id));
}
