import type { AppErrorKind } from './types';

const KINDS: readonly AppErrorKind[] = [
  'io',
  'json',
  'invalidTheme',
  'notFound',
  'invalidInput',
  'pty',
  'conflict',
  'tooLarge',
  'provider',
  'budget',
  'internal',
];

/** Error thrown by every IPC wrapper. Mirrors `AppError` in src-tauri/src/error.rs. */
export class AppError extends Error {
  readonly kind: AppErrorKind;
  /** File size in bytes, for `tooLarge` errors. */
  readonly size: number | undefined;

  constructor(kind: AppErrorKind, message: string, size?: number) {
    super(message);
    this.name = 'AppError';
    this.kind = kind;
    this.size = size;
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
    return new AppError(err.kind, err.message, typeof err.size === 'number' ? err.size : undefined);
  }
  if (err instanceof Error) return new AppError('internal', err.message);
  return new AppError('internal', String(err));
}
