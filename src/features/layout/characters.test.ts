// @vitest-environment jsdom
import { createDockview, type DockviewApi } from 'dockview-core';
import { beforeAll, describe, expect, it } from 'vitest';
import { ensureCharacters, forgetCharacter } from './characters';
import { parsePanelParams } from './panels';

class NoResize {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

beforeAll(() => {
  (globalThis as unknown as { ResizeObserver: typeof NoResize }).ResizeObserver = NoResize;
});

function dock(): DockviewApi {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const api = createDockview(host, {
    createComponent: () => ({ element: document.createElement('div'), init: () => undefined }),
  });
  api.onDidRemovePanel((panel) => forgetCharacter(panel.id));
  return api;
}

const chat = (index: number) => ({
  kind: 'chat',
  index,
  config: { conversationId: `c${index}`, workspace: null, title: '', character: null },
});

function characterId(api: DockviewApi, id: string): string | undefined {
  const params = parsePanelParams(api.getPanel(id)?.params);
  return params?.kind === 'chat' ? params.config.character?.id : undefined;
}

describe('agent panel characters', () => {
  it('survive moves, splits, a lost config and a layout restore', async () => {
    const api = dock();
    api.addPanel({ id: 'chat-1', component: 'panel', params: chat(1) });
    api.addPanel({
      id: 'chat-2',
      component: 'panel',
      params: chat(2),
      position: { direction: 'right' },
    });
    await ensureCharacters(api);
    const first = characterId(api, 'chat-1');
    const second = characterId(api, 'chat-2');
    expect(first).toBeTruthy();
    expect(second).not.toBe(first);

    const panel = api.getPanel('chat-1');
    panel?.api.moveTo({ group: api.getPanel('chat-2')?.group, position: 'center' });
    panel?.api.moveTo({ group: panel.group, position: 'bottom' });
    await ensureCharacters(api);
    expect(characterId(api, 'chat-1')).toBe(first);

    // A stale write drops the character: the panel gets the same one back, not a new pick.
    const params = parsePanelParams(panel?.params);
    if (params?.kind !== 'chat') throw new Error('chat params');
    panel?.api.updateParameters({ ...params, config: { ...params.config, character: null } });
    await ensureCharacters(api);
    expect(characterId(api, 'chat-1')).toBe(first);

    const saved = api.toJSON();
    api.clear();
    api.fromJSON(saved);
    await ensureCharacters(api);
    expect([characterId(api, 'chat-1'), characterId(api, 'chat-2')]).toEqual([first, second]);
  });

  it('gives a genuinely new panel a new character', async () => {
    const api = dock();
    api.addPanel({ id: 'chat-7', component: 'panel', params: chat(7) });
    await ensureCharacters(api);
    const old = characterId(api, 'chat-7');
    api.getPanel('chat-7')?.api.close();
    api.addPanel({ id: 'chat-8', component: 'panel', params: chat(8) });
    await ensureCharacters(api);
    expect(characterId(api, 'chat-8')).toBeTruthy();
    expect(characterId(api, 'chat-8')).not.toBe(old);
  });
});
