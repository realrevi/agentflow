// The handoff protocol: a manager ends every reply with exactly one fenced block,
//   ```agentflow-send to="<worker name>"   (the prompt for that worker)
//   ```agentflow-ask                        (a question for the user)
//   ```agentflow-done                       (the work is finished)
// Fences are scanned like CommonMark: a block inside another code block (an example in a
// ```markdown fence) is content, not a directive. No block or a malformed one never guesses:
// the workstream pauses and asks the user. Several blocks: only the last counts (warning).

export type Directive =
  | { kind: 'send'; to: string; body: string }
  | { kind: 'ask'; body: string }
  | { kind: 'done'; body: string };

export type ParseResult =
  | { ok: true; directive: Directive; multiple: boolean }
  | { ok: false; error: 'none' | 'malformed'; detail: string };

interface Fence {
  char: '`' | '~';
  length: number;
  info: string;
  body: string[];
}

const OPEN = /^ {0,3}(`{3,}|~{3,})(.*)$/;
const CLOSE = /^ {0,3}(`{3,}|~{3,})\s*$/;
const INFO = /^agentflow-([a-z]+)(.*)$/;
const TO = /\bto\s*=\s*(?:"([^"]*)"|'([^']*)'|(\S+))/;

/** Top-level fenced blocks of `text` (unclosed ones are returned with `closed: false`). */
function fences(text: string): { fence: Fence; closed: boolean }[] {
  const out: { fence: Fence; closed: boolean }[] = [];
  let open: Fence | null = null;
  for (const line of text.split(/\r?\n/)) {
    if (open) {
      const close = CLOSE.exec(line);
      const marker = close?.[1];
      if (marker && marker[0] === open.char && marker.length >= open.length) {
        out.push({ fence: open, closed: true });
        open = null;
      } else {
        open.body.push(line);
      }
      continue;
    }
    const start = OPEN.exec(line);
    const marker = start?.[1];
    if (!start || !marker) continue;
    const info = (start[2] ?? '').trim();
    // A backtick fence's info string cannot contain backticks (that is inline code).
    if (marker[0] === '`' && info.includes('`')) continue;
    open = { char: marker[0] === '`' ? '`' : '~', length: marker.length, info, body: [] };
  }
  if (open) out.push({ fence: open, closed: false });
  return out;
}

export function parseReply(text: string): ParseResult {
  const blocks = fences(text).filter(({ fence }) => fence.info.startsWith('agentflow'));
  const last = blocks.at(-1);
  if (!last) return { ok: false, error: 'none', detail: '' };
  const { fence, closed } = last;
  const multiple = blocks.length > 1;
  if (!closed) return { ok: false, error: 'malformed', detail: 'unclosed' };
  const info = INFO.exec(fence.info);
  const body = fence.body.join('\n').trim();
  switch (info?.[1]) {
    case 'send': {
      const match = TO.exec(info[2] ?? '');
      const to = (match?.[1] ?? match?.[2] ?? match?.[3] ?? '').trim();
      if (!to) return { ok: false, error: 'malformed', detail: 'noWorker' };
      if (!body) return { ok: false, error: 'malformed', detail: 'emptyPrompt' };
      return { ok: true, directive: { kind: 'send', to, body }, multiple };
    }
    case 'ask':
      return { ok: true, directive: { kind: 'ask', body }, multiple };
    case 'done':
      return { ok: true, directive: { kind: 'done', body }, multiple };
    default:
      return { ok: false, error: 'malformed', detail: fence.info.slice(0, 40) };
  }
}

/** The worker a `send` names: case-insensitive, workers of this workstream only. */
export function resolveWorker(
  name: string,
  workers: readonly { id: string; name: string }[],
): string | null {
  const wanted = name.trim().toLowerCase();
  return workers.find((worker) => worker.name.trim().toLowerCase() === wanted)?.id ?? null;
}
