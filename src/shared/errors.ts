import type { AppErrorKind } from './types';

const KINDS: readonly AppErrorKind[] = ['io', 'json', 'invalidTheme', 'notFound', 'internal'];

/** Error thrown by every IPC wrapper. Mirrors `AppError` in src-tauri/src/error.rs. */
export class AppError extends Error {
  readonly kind: AppErrorKind;

  constructor(kind: AppErrorKind, message: string) {
    super(message);
    this.name = 'AppError';
    this.kind = kind;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isKind(value: unknown): value is AppErrorKind {
  return typeof value === 'string' && (KINDS as readonly string[]).includes(value);
}

/** Normalizes anything thrown (backend payloads, strings, JS errors) into an `AppError`. */
export function toAppError(err: unknown): AppError {
  if (err instanceof AppError) return err;
  if (isRecord(err) && isKind(err.kind) && typeof err.message === 'string') {
    return new AppError(err.kind, err.message);
  }
  if (err instanceof Error) return new AppError('internal', err.message);
  return new AppError('internal', String(err));
}
