// @vitest-environment jsdom
// Shared components: rendering and the key keyboard behavior.
import { createSignal, type JSX } from 'solid-js';
import { render } from 'solid-js/web';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Button, IconButton } from '../Button';
import { Dialog, trapTab } from '../Dialog';
import { Toggle } from '../Field';
import { Menu } from '../Menu';
import { SegmentedControl, stepOption, Tabs } from '../Segmented';

// jsdom has no ResizeObserver (the segmented indicator follows size changes with it).
globalThis.ResizeObserver ??= class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
};

let dispose: (() => void) | undefined;
afterEach(() => {
  dispose?.();
  dispose = undefined;
  document.body.innerHTML = '';
});

function mount(ui: () => JSX.Element): HTMLElement {
  const root = document.createElement('div');
  document.body.append(root);
  dispose = render(ui, root);
  return root;
}

function key(target: Element, name: string, shift = false): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key: name,
    shiftKey: shift,
    bubbles: true,
    cancelable: true,
  });
  target.dispatchEvent(event);
  return event;
}

const active = (): Element => document.activeElement ?? document.body;

describe('Button and IconButton', () => {
  it('maps variants and sizes to classes and labels icon-only buttons', () => {
    const root = mount(() => (
      <>
        <Button variant="secondary" size="sm">
          Go
        </Button>
        <Button variant="danger" label="Delete" icon="trash" />
        <IconButton icon="x" label="Close" />
      </>
    ));
    const [secondary, danger, icon] = [...root.querySelectorAll('button')];
    expect(secondary?.classList.contains('af-button')).toBe(true);
    expect(secondary?.classList.contains('secondary')).toBe(true);
    expect(secondary?.classList.contains('small')).toBe(true);
    expect(secondary?.getAttribute('type')).toBe('button');
    expect(danger?.classList.contains('icon-only')).toBe(true);
    expect(danger?.getAttribute('aria-label')).toBe('Delete');
    expect(icon?.getAttribute('aria-label')).toBe('Close');
    expect(icon?.querySelector('svg')).not.toBeNull();
  });
});

describe('Toggle', () => {
  it('is a switch that reports its state', () => {
    const onChange = vi.fn();
    const root = mount(() => (
      <Toggle checked={false} onChange={(e) => onChange(e.currentTarget.checked)} />
    ));
    const input = root.querySelector('input');
    expect(input?.getAttribute('role')).toBe('switch');
    input?.click();
    expect(onChange).toHaveBeenCalledWith(true);
  });
});

describe('SegmentedControl and Tabs', () => {
  const options = [
    { value: 'a', label: 'A' },
    { value: 'b', label: 'B', disabled: true },
    { value: 'c', label: 'C' },
  ] as const;

  it('steps over disabled options and wraps around', () => {
    expect(stepOption(options, 'a', 'ArrowRight')).toBe('c');
    expect(stepOption(options, 'c', 'ArrowRight')).toBe('a');
    expect(stepOption(options, 'a', 'ArrowLeft')).toBe('c');
    expect(stepOption(options, 'c', 'Home')).toBe('a');
    expect(stepOption(options, 'a', 'End')).toBe('c');
    expect(stepOption(options, 'a', 'Enter')).toBeNull();
  });

  it('selects with arrow keys and keeps only the active item tabbable', () => {
    const [value, setValue] = createSignal<'a' | 'b' | 'c'>('a');
    const root = mount(() => (
      <SegmentedControl label="Mode" value={value()} onChange={setValue} options={options} />
    ));
    const group = root.querySelector('[role="radiogroup"]');
    expect(group?.getAttribute('aria-label')).toBe('Mode');
    const buttons = () => [...root.querySelectorAll('button')];
    expect(buttons().map((b) => b.tabIndex)).toEqual([0, -1, -1]);
    if (group) key(group, 'ArrowRight');
    expect(value()).toBe('c');
    expect(buttons()[2]?.getAttribute('aria-checked')).toBe('true');
    expect(buttons().map((b) => b.tabIndex)).toEqual([-1, -1, 0]);
  });

  it('renders tabs with aria-selected', () => {
    const root = mount(() => (
      <Tabs label="Tabs" value="c" onChange={() => undefined} options={options} />
    ));
    expect(root.querySelector('[role="tablist"]')).not.toBeNull();
    expect(root.querySelector('[aria-selected="true"]')?.textContent).toBe('C');
  });
});

describe('Menu', () => {
  it('moves with arrow keys, skips disabled items and closes on Escape', () => {
    const anchor = document.createElement('button');
    document.body.append(anchor);
    const onClose = vi.fn();
    const onSelect = vi.fn();
    mount(() => (
      <Menu
        anchor={anchor}
        placement="below-start"
        onClose={onClose}
        onSelect={onSelect}
        items={[
          { id: 'one', label: 'One' },
          { id: 'two', label: 'Two', disabled: true },
          { id: 'three', label: 'Three' },
        ]}
      />
    ));
    const items = [...document.querySelectorAll<HTMLButtonElement>('.af-menu-item')];
    expect(document.activeElement).toBe(items[0]);
    key(active(), 'ArrowDown');
    expect(document.activeElement).toBe(items[2]);
    key(active(), 'ArrowDown');
    expect(document.activeElement).toBe(items[0]);
    key(active(), 'End');
    expect(document.activeElement).toBe(items[2]);
    items[2]?.click();
    expect(onSelect).toHaveBeenCalledWith('three');
    key(document.body, 'Escape');
    expect(onClose).toHaveBeenCalled();
  });
});

describe('Dialog', () => {
  it('traps Tab inside, closes on Escape and restores focus', () => {
    const opener = document.createElement('button');
    document.body.append(opener);
    opener.focus();
    const [open, setOpen] = createSignal(true);
    const onClose = vi.fn(() => setOpen(false));
    mount(() => (
      <Dialog open={open()} title="Rename" onClose={onClose} actions={<Button>OK</Button>}>
        <input id="first" />
      </Dialog>
    ));
    const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    const input = document.getElementById('first');
    const ok = dialog?.querySelector('button');
    expect(document.activeElement).toBe(input);
    ok?.focus();
    expect(key(active(), 'Tab').defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(input);
    key(active(), 'Tab', true);
    expect(document.activeElement).toBe(ok);
    key(active(), 'Escape');
    expect(onClose).toHaveBeenCalled();
  });

  it('trapTab cycles from the last focusable element to the first', () => {
    const box = document.createElement('div');
    box.innerHTML = '<button id="a">a</button><button id="b">b</button>';
    document.body.append(box);
    box.querySelector<HTMLButtonElement>('#b')?.focus();
    const event = new KeyboardEvent('keydown', { key: 'Tab', cancelable: true });
    expect(trapTab(box, event)).toBe(true);
    expect(document.activeElement?.id).toBe('a');
  });
});
