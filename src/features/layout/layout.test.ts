import type { DockviewApi } from 'dockview-core';
import { beforeEach, describe, expect, it } from 'vitest';
import { buildDefaultLayout } from './defaultLayout';
import { groupForRole } from './placement';
import { groupRole, replaceRoles, roleForPanel, snapshotRoles } from './roles';
import { LAYOUT_VERSION, parseSavedLayout, serializeLayout } from './savedLayout';

interface FakePanel {
  id: string;
  params: Record<string, unknown>;
}

/** Just enough of dockview to record where groups and panels go. */
class FakeDock {
  width = 1000;
  height = 800;
  groups: { id: string; panels: FakePanel[]; options: unknown; sizes: unknown[] }[] = [];
  active: string | undefined;
  private nextId = 1;

  addGroup(options?: unknown) {
    const group = {
      id: String(this.nextId++),
      panels: [] as FakePanel[],
      options,
      sizes: [] as unknown[],
      api: {
        setActive: () => (this.active = group.id),
        setSize: (size: unknown) => group.sizes.push(size),
      },
    };
    this.groups.push(group);
    return group;
  }

  addPanel(options: {
    id: string;
    params: Record<string, unknown>;
    position: { referenceGroup: string };
  }) {
    this.getGroup(options.position.referenceGroup)?.panels.push(options);
  }

  getGroup(id: string) {
    return this.groups.find((group) => group.id === id);
  }

  asApi(): DockviewApi {
    return this as unknown as DockviewApi;
  }
}

describe('saved layout', () => {
  it('replaces phase 1–2 layouts (Workspace placeholder) with the default', () => {
    const phase2 = {
      grid: {
        root: { type: 'branch', data: [] },
        width: 800,
        height: 600,
        orientation: 'HORIZONTAL',
      },
      panels: { workspace: { id: 'workspace', contentComponent: 'workspace' } },
      activeGroup: '1',
    };
    expect(parseSavedLayout(phase2)).toBeNull();
    expect(parseSavedLayout({ version: 1, dock: phase2, roles: {} })).toBeNull();
    expect(parseSavedLayout(null)).toBeNull();
  });

  it('round-trips the current version and drops unknown roles', () => {
    const dock = { grid: { root: {} }, panels: {} } as never;
    const saved = serializeLayout(dock, { '1': 'agents', '2': 'files' });
    expect(saved['version']).toBe(LAYOUT_VERSION);
    expect(parseSavedLayout(saved)).toEqual({ dock, roles: { '1': 'agents', '2': 'files' } });
    const tampered = { ...saved, roles: { '1': 'agents', '2': 'kitchen' } };
    expect(parseSavedLayout(tampered)?.roles).toEqual({ '1': 'agents' });
  });
});

describe('default layout', () => {
  beforeEach(() => replaceRoles({}));

  it('builds the agent area, files on the right and a shell below the center', () => {
    const dock = new FakeDock();
    buildDefaultLayout(dock.asApi());
    const [agents, files, terminal] = dock.groups;
    expect(snapshotRoles()).toEqual({ '1': 'agents', '2': 'files', '3': 'terminal' });
    expect(agents?.panels).toEqual([]);
    expect(files?.options).toEqual({ direction: 'right' });
    expect(files?.sizes).toEqual([{ width: 220 }]);
    expect(files?.panels.map((panel) => panel.id)).toEqual(['files']);
    expect(terminal?.options).toEqual({ referenceGroup: agents, direction: 'below' });
    expect(terminal?.sizes).toEqual([{ height: 240 }]);
    expect(terminal?.panels[0]?.params).toMatchObject({ kind: 'terminal', index: 1 });
    expect(dock.active).toBe('1');
  });

  it('recreates a closed role group at its default position', () => {
    const dock = new FakeDock();
    buildDefaultLayout(dock.asApi());
    dock.groups = dock.groups.filter((group) => groupRole(group.id) !== 'files');
    const files = groupForRole(dock.asApi(), 'files');
    expect(groupRole(files.id)).toBe('files');
    expect(dock.groups.at(-1)?.options).toEqual({ direction: 'right' });
    expect(groupForRole(dock.asApi(), 'terminal').id).toBe('3');
  });

  it('opens shells at the bottom and everything else in the center', () => {
    const shell = { kind: 'shell' as const, command: null, args: [], cwd: null };
    expect(roleForPanel({ kind: 'terminal', index: 1, config: shell })).toBe('terminal');
    expect(roleForPanel({ kind: 'terminal', index: 2, config: { ...shell, kind: 'claude' } })).toBe(
      'agents',
    );
    expect(roleForPanel({ kind: 'editor', index: 1, root: '/w', path: 'a.ts' })).toBe('agents');
    expect(roleForPanel({ kind: 'files' })).toBe('files');
  });
});
