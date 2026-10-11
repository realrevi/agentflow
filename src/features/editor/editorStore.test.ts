import { describe, expect, it } from 'vitest';
import { dirtyEditorCount, registerEditor, saveAllEditors, setEditorDirty } from './editorStore';

function editor(id: string, saves: boolean) {
  const saved: string[] = [];
  const unregister = registerEditor(id, {
    fileName: () => `${id}.ts`,
    save: async () => {
      saved.push(id);
      if (saves) setEditorDirty(id, false);
      return saves;
    },
  });
  return { saved, unregister };
}

describe('saveAllEditors', () => {
  it('saves only dirty editors and reports whether all succeeded', async () => {
    const clean = editor('clean', true);
    const a = editor('a', true);
    const b = editor('b', true);
    setEditorDirty('a', true);
    setEditorDirty('b', true);
    expect(dirtyEditorCount()).toBe(2);
    await expect(saveAllEditors()).resolves.toBe(true);
    expect([...clean.saved, ...a.saved, ...b.saved]).toEqual(['a', 'b']);
    expect(dirtyEditorCount()).toBe(0);
    [clean, a, b].forEach((e) => e.unregister());
  });

  it('fails when any save fails, keeping that editor dirty', async () => {
    const ok = editor('ok', true);
    const broken = editor('broken', false);
    setEditorDirty('ok', true);
    setEditorDirty('broken', true);
    await expect(saveAllEditors()).resolves.toBe(false);
    expect(dirtyEditorCount()).toBe(1);
    ok.unregister();
    broken.unregister();
    expect(dirtyEditorCount()).toBe(0);
  });
});
