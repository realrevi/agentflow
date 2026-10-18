import { describe, expect, it } from 'vitest';
import type { DockviewApi } from 'dockview-core';
import { setGroupRole } from './roles';
import { revealGroup, roleVisible, toggleRoleGroups } from './visibility';

function fakeDock(ids: string[]) {
  const visible = new Map(ids.map((id) => [id, true]));
  const group = (id: string) => ({
    id,
    api: {
      get isVisible() {
        return visible.get(id) ?? false;
      },
      setVisible: (on: boolean) => visible.set(id, on),
    },
  });
  const dock = {
    get groups() {
      return ids.map(group);
    },
    getGroup: (id: string) => (ids.includes(id) ? group(id) : undefined),
  } as unknown as DockviewApi;
  return { dock, visible };
}

describe('terminal and Files area toggles', () => {
  it('hides and shows all groups of a role', () => {
    setGroupRole('g-term', 'terminal');
    setGroupRole('g-files', 'files');
    const { dock, visible } = fakeDock(['g-term', 'g-files']);
    expect(toggleRoleGroups(dock, 'terminal')).toBe(true);
    expect(visible.get('g-term')).toBe(false);
    expect(visible.get('g-files')).toBe(true);
    expect(roleVisible('terminal')).toBe(false);
    expect(roleVisible('files')).toBe(true);
    toggleRoleGroups(dock, 'terminal');
    expect(visible.get('g-term')).toBe(true);
    expect(roleVisible('terminal')).toBe(true);
  });

  it('reports a missing group so the caller can open one', () => {
    const { dock } = fakeDock([]);
    expect(toggleRoleGroups(dock, 'files')).toBe(false);
  });

  it('shows a hidden group again before a panel opens in it', () => {
    setGroupRole('g-term2', 'terminal');
    const { dock, visible } = fakeDock(['g-term2']);
    toggleRoleGroups(dock, 'terminal');
    expect(visible.get('g-term2')).toBe(false);
    revealGroup(dock, 'g-term2');
    expect(visible.get('g-term2')).toBe(true);
    expect(roleVisible('terminal')).toBe(true);
  });
});
