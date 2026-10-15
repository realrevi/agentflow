// Smooth reveal of streamed text. Deltas arrive in bursts; each frame shows a number of
// characters that grows with the backlog, and anything that arrived more than MAX_LAG_MS
// ago is shown at once, so the display never trails the stream by more than ~300 ms.

export const MAX_LAG_MS = 300;
/** Pace when there is little backlog: smooth, but never slower than this. */
const BASE_CHARS_PER_SECOND = 90;
/** The backlog drains in about this long (well inside the lag bound). */
const DRAIN_MS = MAX_LAG_MS / 3;
/** One frame of slack, so the deadline is met even when the next frame is late-ish. */
const FRAME_SLACK_MS = 34;

export class RevealBuffer {
  private text = '';
  private shown = 0;
  private carry = 0;
  private lastStep: number | null = null;
  /** Cumulative end offsets of pushed deltas with their arrival time, oldest first. */
  private arrivals: { end: number; at: number }[] = [];

  push(delta: string, now: number): void {
    if (!delta) return;
    this.text += delta;
    this.arrivals.push({ end: this.text.length, at: now });
  }

  /** Characters not shown yet. */
  get backlog(): number {
    return this.text.length - this.shown;
  }

  /** Advances to `now`; returns the visible text. */
  step(now: number): string {
    const dt = this.lastStep === null ? 16 : Math.max(0, now - this.lastStep);
    this.lastStep = now;
    if (this.backlog <= 0) {
      this.carry = 0;
      return this.visible();
    }
    let due = this.shown;
    while (this.arrivals.length > 0) {
      const first = this.arrivals[0];
      if (!first || first.at > now - (MAX_LAG_MS - FRAME_SLACK_MS)) break;
      due = Math.max(due, first.end);
      this.arrivals.shift();
    }
    const rate = Math.max(BASE_CHARS_PER_SECOND / 1000, this.backlog / DRAIN_MS);
    this.carry += rate * dt;
    const smooth = Math.floor(this.carry);
    this.carry -= smooth;
    this.shown = Math.min(this.text.length, Math.max(due, this.shown + smooth));
    return this.visible();
  }

  /** Shows everything (stream ended and the user should see it all, or reveal is off). */
  flush(): string {
    this.shown = this.text.length;
    this.arrivals = [];
    return this.text;
  }

  visible(): string {
    let end = this.shown;
    // Never cut a surrogate pair (emoji) in half.
    const code = this.text.charCodeAt(end - 1);
    if (end < this.text.length && code >= 0xd800 && code <= 0xdbff) end += 1;
    return this.text.slice(0, end);
  }

  get full(): string {
    return this.text;
  }
}
