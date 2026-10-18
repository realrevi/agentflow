import { describe, expect, it } from 'vitest';
import { cutBytes, reportMessage } from './messages';
import { parseReply, resolveWorker } from './protocol';

const fence = '```';

describe('handoff protocol parser', () => {
  it('reads send, ask and done blocks', () => {
    const send = parseReply(
      `I will split the work.\n\n${fence}agentflow-send to="Fox"\nAdd a test for sum().\nReport the result.\n${fence}\n`,
    );
    expect(send).toEqual({
      ok: true,
      multiple: false,
      directive: { kind: 'send', to: 'Fox', body: 'Add a test for sum().\nReport the result.' },
    });
    expect(parseReply(`${fence}agentflow-send to=Owl\nGo.\n${fence}`)).toMatchObject({
      ok: true,
      directive: { to: 'Owl' },
    });
    expect(parseReply(`Question:\n${fence}agentflow-ask\nWhich port?\n${fence}`)).toMatchObject({
      ok: true,
      directive: { kind: 'ask', body: 'Which port?' },
    });
    expect(parseReply(`~~~agentflow-done\nAll green.\n~~~`)).toMatchObject({
      ok: true,
      directive: { kind: 'done', body: 'All green.' },
    });
  });

  it('pauses on no block or a malformed one, never guesses', () => {
    expect(parseReply('Done, the tests pass.')).toEqual({ ok: false, error: 'none', detail: '' });
    expect(parseReply(`${fence}agentflow-send\nNo worker named.\n${fence}`)).toMatchObject({
      ok: false,
      error: 'malformed',
      detail: 'noWorker',
    });
    expect(parseReply(`${fence}agentflow-send to="Fox"\n\n${fence}`)).toMatchObject({
      error: 'malformed',
      detail: 'emptyPrompt',
    });
    expect(parseReply(`${fence}agentflow-sned to="Fox"\nx\n${fence}`)).toMatchObject({
      error: 'malformed',
    });
    expect(parseReply(`${fence}agentflow-send to="Fox"\nnever closed`)).toMatchObject({
      error: 'malformed',
      detail: 'unclosed',
    });
  });

  it('only the last of several blocks counts, with a warning', () => {
    const reply = `${fence}agentflow-send to="Fox"\nfirst\n${fence}\nthen\n${fence}agentflow-ask\nOk?\n${fence}`;
    expect(parseReply(reply)).toMatchObject({
      ok: true,
      multiple: true,
      directive: { kind: 'ask', body: 'Ok?' },
    });
  });

  it('a block inside a normal code block is an example, not a directive', () => {
    const quad = '````';
    const example = `Use this format:\n${quad}markdown\n${fence}agentflow-send to="Fox"\nexample\n${fence}\n${quad}\nNo decision yet.`;
    expect(parseReply(example)).toMatchObject({ ok: false, error: 'none' });
    const triple = `${fence}md\n${fence}agentflow-done\nnot real\n${fence}\n${fence}\n`;
    expect(parseReply(triple).ok).toBe(false);
    const inline = 'Write `agentflow-send` blocks like ```agentflow-send to="x"``` inline.';
    expect(parseReply(inline)).toMatchObject({ ok: false, error: 'none' });
  });

  it('resolves worker names case-insensitively, inside the workstream only', () => {
    const workers = [
      { id: 'terminal-2', name: 'Fox' },
      { id: 'chat-3', name: 'Owl' },
    ];
    expect(resolveWorker(' fox ', workers)).toBe('terminal-2');
    expect(resolveWorker('Panda', workers)).toBeNull();
  });
});

describe('worker reports', () => {
  it('formats the report and caps it with a note; the full text is kept', () => {
    const short = reportMessage('Fox', 2, 'Tests pass.', 30 * 1024);
    expect(short.text).toBe('Report from Fox (turn 2):\n\nTests pass.');
    expect(short.truncated).toBe(false);
    const long = 'x'.repeat(5000);
    const capped = reportMessage('Fox', 3, long, 1024);
    expect(capped.truncated).toBe(true);
    expect(capped.full).toBe(`Report from Fox (turn 3):\n\n${long}`);
    expect(capped.text.length).toBeLessThan(1400);
    expect(capped.text).toContain('[Report truncated by AgentFlow: 1 of 5 KB shown.');
    expect(cutBytes('aé😀b', 5)).toBe('aé');
    expect(reportMessage('Fox', 1, '  ', 1024).text).toContain('without a final message');
  });
});
