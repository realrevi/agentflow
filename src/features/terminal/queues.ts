// Keeps IPC calls for one session in order. Tauri may run concurrent async commands
// out of order, so typed input is sent one call at a time; keystrokes typed while a call
// is in flight are batched into the next one.

type Send<T> = (value: T) => Promise<unknown>;

export class InputQueue {
  private pending = '';
  private busy = false;

  constructor(
    private readonly send: Send<string>,
    private readonly onError: (err: unknown) => void,
  ) {}

  push(data: string): void {
    this.pending += data;
    if (!this.busy) void this.drain();
  }

  private async drain(): Promise<void> {
    this.busy = true;
    while (this.pending) {
      const chunk = this.pending;
      this.pending = '';
      try {
        await this.send(chunk);
      } catch (err) {
        this.pending = '';
        this.onError(err);
      }
    }
    this.busy = false;
  }
}

/** Like InputQueue, but only the newest value matters (e.g. terminal size). */
export class LatestQueue<T> {
  private next: T | undefined;
  private busy = false;

  constructor(
    private readonly send: Send<T>,
    private readonly onError: (err: unknown) => void,
  ) {}

  push(value: T): void {
    this.next = value;
    if (!this.busy) void this.drain();
  }

  private async drain(): Promise<void> {
    this.busy = true;
    while (this.next !== undefined) {
      const value = this.next;
      this.next = undefined;
      try {
        await this.send(value);
      } catch (err) {
        this.onError(err);
      }
    }
    this.busy = false;
  }
}

/**
 * Flow control: counts output bytes xterm has processed and says when to acknowledge them
 * (in chunks, to keep IPC calls rare). The chunk must stay below the backend's high-water mark.
 */
export class AckBatcher {
  private pending = 0;

  constructor(private readonly chunk: number) {}

  /** Returns the number of bytes to acknowledge now, or 0 to keep batching. */
  add(bytes: number, canSend: boolean): number {
    this.pending += bytes;
    if (!canSend || this.pending < this.chunk) return 0;
    const ready = this.pending;
    this.pending = 0;
    return ready;
  }
}
