// @vitest-environment jsdom
// Settings > About: version, release notes link, the update check and the licenses dialog.
import { render } from 'solid-js/web';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { shortNotes } from '../../updates';
import { AboutSection, releaseNotesUrl } from './AboutSection';

const ipc = vi.hoisted(() => ({
  openUrl: vi.fn(async () => {}),
  updateCheck: vi.fn(async () => null),
  updateInstall: vi.fn(async () => {}),
  saveSettings: vi.fn(async (settings: unknown) => settings),
}));
vi.mock('../../../shared/ipc', () => ({ ipc }));

let dispose: (() => void) | undefined;
afterEach(() => {
  dispose?.();
  dispose = undefined;
  document.body.innerHTML = '';
  vi.clearAllMocks();
});

function mount(): HTMLElement {
  const root = document.createElement('div');
  document.body.append(root);
  dispose = render(() => <AboutSection />, root);
  return root;
}

function button(root: HTMLElement, text: string): HTMLButtonElement {
  const found = [...root.querySelectorAll('button')].find((b) => b.textContent?.includes(text));
  if (!found) throw new Error(`no "${text}" button`);
  return found;
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('About section', () => {
  it('shows the version and opens its release notes', () => {
    const root = mount();
    expect(root.textContent).toContain(`AgentFlow ${__APP_VERSION__}`);
    button(root, 'Release notes').click();
    expect(ipc.openUrl).toHaveBeenCalledWith(releaseNotesUrl(__APP_VERSION__));
    expect(releaseNotesUrl('1.2.3')).toBe(
      'https://github.com/realrevi/agentflow/releases/tag/v1.2.3',
    );
  });

  it('checks for updates by hand and says when this is the latest version', async () => {
    const root = mount();
    button(root, 'Check for updates').click();
    await settle();
    expect(ipc.updateCheck).toHaveBeenCalledOnce();
    expect(ipc.updateInstall).not.toHaveBeenCalled();
    expect(root.querySelector('[role="status"]')?.textContent).toContain('latest version');
  });

  it('opens the third-party licenses', async () => {
    const root = mount();
    button(root, 'Third-party licenses').click();
    await vi.waitFor(() => {
      const text = document.querySelector('.af-licenses-text')?.textContent ?? '';
      expect(text).toContain('Third-party notices');
      expect(text).toContain('Lucide');
    });
  });
});

describe('update notes in the toast', () => {
  it('keeps the first lines, without markdown marks, and cuts long notes', () => {
    expect(shortNotes(null)).toBeUndefined();
    expect(shortNotes('### Added\n\n- **Faster** start\n- Fixes')).toBe(
      'Added · Faster start · Fixes',
    );
    const long = shortNotes('x'.repeat(500), 20);
    expect(long).toHaveLength(20);
    expect(long?.endsWith('…')).toBe(true);
  });
});
