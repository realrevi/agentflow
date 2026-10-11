import { describe, expect, it } from 'vitest';
import { chatTitle, newChatConfig, parseChatConfig } from './config';

describe('chat panel config', () => {
  it('round-trips and repairs persisted configs', () => {
    const config = {
      ...newChatConfig('c18f2a', '/work'),
      title: 'Refactor',
      character: { id: 'fox', palette: 2 },
    };
    expect(parseChatConfig(JSON.parse(JSON.stringify(config)))).toEqual(config);
    expect(
      parseChatConfig({
        conversationId: 'c1',
        workspace: '  ',
        character: { id: 'x', palette: -1 },
      }),
    ).toEqual({
      conversationId: 'c1',
      workspace: null,
      title: '',
      character: null,
      historyKey: null,
    });
    expect(parseChatConfig({ conversationId: '../etc' })).toBeNull();
    expect(parseChatConfig('chat')).toBeNull();
  });

  it('shortens long titles', () => {
    const title = chatTitle({ title: 'x'.repeat(80) });
    expect(title.length).toBe(40);
    expect(title.endsWith('…')).toBe(true);
  });
});
