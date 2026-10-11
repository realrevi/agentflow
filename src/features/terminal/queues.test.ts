import { describe, expect, it } from 'vitest';
import { AckBatcher, InputQueue, LatestQueue } from './queues';

function deferred() {
  let resolve = () => {};
  const promise = new Promise<void>((r) => (resolve = r));
  return { promise, resolve };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('InputQueue', () => {
  it('sends in order, one call at a time, batching input typed meanwhile', async () => {
    const sent: string[] = [];
    let gate = deferred();
    const queue = new InputQueue(
      async (data) => {
        sent.push(data);
        await gate.promise;
      },
      () => {},
    );

    queue.push('l');
    queue.push('s');
    queue.push('\r');
    expect(sent).toEqual(['l']);
    const first = gate;
    gate = deferred();
    first.resolve();
    await tick();
    expect(sent).toEqual(['l', 's\r']);
    gate.resolve();
    await tick();
    queue.push('x');
    await tick();
    expect(sent).toEqual(['l', 's\r', 'x']);
  });

  it('reports errors and keeps working', async () => {
    const errors: unknown[] = [];
    let fail = true;
    const sent: string[] = [];
    const queue = new InputQueue(
      async (data) => {
        if (fail) throw new Error('gone');
        sent.push(data);
      },
      (err) => errors.push(err),
    );
    queue.push('a');
    await tick();
    fail = false;
    queue.push('b');
    await tick();
    expect(errors).toHaveLength(1);
    expect(sent).toEqual(['b']);
  });
});

describe('LatestQueue', () => {
  it('skips stale values while a call is in flight', async () => {
    const sent: number[] = [];
    const gate = deferred();
    const queue = new LatestQueue<number>(
      async (value) => {
        sent.push(value);
        if (value === 1) await gate.promise;
      },
      () => {},
    );
    queue.push(1);
    queue.push(2);
    queue.push(3);
    gate.resolve();
    await tick();
    expect(sent).toEqual([1, 3]);
  });
});

describe('AckBatcher', () => {
  it('acknowledges in chunks and holds bytes until it may send', () => {
    const acks = new AckBatcher(100);
    expect(acks.add(60, true)).toBe(0);
    expect(acks.add(50, true)).toBe(110);
    expect(acks.add(150, false)).toBe(0); // session id not known yet
    expect(acks.add(0, true)).toBe(150);
  });
});
