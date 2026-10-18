// The texts a workstream sends to its agents (model-facing, so not translated): the
// Workstream Manager instructions (an API manager's system prompt addition, a CLI
// manager's one-time setup message) and worker reports, capped with a note.

export interface MemberInfo {
  name: string;
  /** "Claude Code", "API agent (glm-5.3)", ... */
  kind: string;
}

/** The built-in Workstream Manager instructions. */
export function managerInstructions(workstream: string, workers: readonly MemberInfo[]): string {
  const list = workers.map((worker) => `- ${worker.name} (${worker.kind})`).join('\n');
  return `## Workstream Manager

You manage the workstream "${workstream}" in AgentFlow: you plan the work, hand focused tasks to your workers and decide the next step from their reports. Your workers:
${list || '- (none yet)'}

End EVERY reply with exactly one AgentFlow block, as the very last thing in the reply:

\`\`\`agentflow-send to="<worker name>"
The complete prompt for that worker.
\`\`\`

\`\`\`agentflow-ask
Your question for the user.
\`\`\`

\`\`\`agentflow-done
A short summary of what was done.
\`\`\`

Rules:
- The block is plain text at the end of your final message. There is no tool for handoffs: writing the block is what hands the work over, so never stop with "handing it over now" and no block.
- One block per reply, at the end. Without a valid block AgentFlow pauses and asks the user.
- Send only to the workers listed above, by name.
- A worker prompt must be focused and complete: the worker sees only what you send. Say the goal, the files involved, constraints, how to check the result and what to report back.
- A worker's report arrives as a message starting with "Report from <worker> (turn N):". Read it, then send the next task, ask, or finish.
- Ask the user first (agentflow-ask) before merges, pushes, deleting files or branches, dependency changes and database or schema migrations. Never tell a worker to do these without the user's yes.
- Use agentflow-done when the task is finished or cannot go on.`;
}

/** The one-time setup message for a CLI manager. */
export function setupMessage(workstream: string, workers: readonly MemberInfo[]): string {
  return `AgentFlow setup: from now on you are the manager of a workstream. Keep these instructions for the whole session.

${managerInstructions(workstream, workers)}

Reply now with an agentflow-ask block that asks the user for the task (or an agentflow-send block if the task is already clear).`;
}

/** Sent when the user asks AgentFlow to remind a manager of the format. */
export const FORMAT_REMINDER =
  'AgentFlow: your last reply had no valid AgentFlow block, so nothing was handed over. Reply again and end it with exactly one block: agentflow-send to="<worker name>" (the full prompt), agentflow-ask (a question for the user) or agentflow-done (a summary).';

const encoder = new TextEncoder();

/** `text` cut to at most `maxBytes` UTF-8 bytes, on a character boundary. */
export function cutBytes(text: string, maxBytes: number): string {
  if (encoder.encode(text).length <= maxBytes) return text;
  let low = 0;
  let high = text.length;
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    if (encoder.encode(text.slice(0, mid)).length <= maxBytes) low = mid;
    else high = mid - 1;
  }
  // Never split a surrogate pair.
  const code = text.charCodeAt(low - 1);
  return text.slice(0, code >= 0xd800 && code <= 0xdbff ? low - 1 : low);
}

export function byteLength(text: string): number {
  return encoder.encode(text).length;
}

/** "Report from <worker> (turn N):" + the report; over `capBytes` it is cut with a note
 * (`full` is what the workstream log keeps). */
export function reportMessage(
  worker: string,
  turn: number,
  report: string,
  capBytes: number,
): { text: string; full: string; truncated: boolean } {
  const header = `Report from ${worker} (turn ${turn}):`;
  const body = report.trim() || '(The worker ended its turn without a final message.)';
  const full = `${header}\n\n${body}`;
  const size = byteLength(body);
  if (size <= capBytes) return { text: full, full, truncated: false };
  const kb = (bytes: number) => Math.round(bytes / 1024);
  const shown = cutBytes(body, capBytes);
  const note = `[Report truncated by AgentFlow: ${kb(byteLength(shown))} of ${kb(size)} KB shown. The full text is in the workstream log; ask the worker if you need the rest.]`;
  return { text: `${header}\n\n${shown}\n\n${note}`, full, truncated: true };
}
