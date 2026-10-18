import { createSignal } from 'solid-js';

export type ToastKind = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  detail?: string;
  /** One button, e.g. "Show" for a panel that waits; clicking it also dismisses. */
  action?: { label: string; run: () => void };
  /** Stays until dismissed (e.g. an update offer), instead of closing on its own. */
  sticky?: boolean;
  /** Set while its exit animation plays. */
  leaving?: boolean;
}

const AUTO_DISMISS_MS: Record<ToastKind, number> = { info: 3500, success: 3500, error: 7000 };
const MAX_TOASTS = 4;
/** The exit animation (statusbar.css); with reduced motion the CSS duration is 0. */
const LEAVE_MS = 160;

const [toasts, setToasts] = createSignal<readonly Toast[]>([]);
let nextId = 1;

export { toasts };

export function showToast(toast: Omit<Toast, 'id'>): void {
  const id = nextId++;
  setToasts((list) => [...list, { ...toast, id }].slice(-MAX_TOASTS));
  if (!toast.sticky) window.setTimeout(() => dismissToast(id), AUTO_DISMISS_MS[toast.kind]);
}

export function dismissToast(id: number): void {
  setToasts((list) => list.map((toast) => (toast.id === id ? { ...toast, leaving: true } : toast)));
  window.setTimeout(() => setToasts((list) => list.filter((toast) => toast.id !== id)), LEAVE_MS);
}
