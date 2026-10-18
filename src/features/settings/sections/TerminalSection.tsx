import { For, onMount, Show } from 'solid-js';
import type { CursorStyle } from '../../../shared/types';
import { t, type I18nKey } from '../../i18n';
import { cliStatus, kindLabel, loadCliStatus } from '../../terminal';
import { TERMINAL_FONT_SIZE, TERMINAL_SCROLLBACK_MAX } from '../defaults';
import { settings, updateSettings } from '../settingsStore';
import { Row, Section } from './Section';
import { Button } from '../../../shared/ui/Button';
import { Input, Select, Toggle } from '../../../shared/ui/Field';

const CURSORS: readonly { id: CursorStyle; label: I18nKey }[] = [
  { id: 'block', label: 'settings.terminal.cursor.block' },
  { id: 'underline', label: 'settings.terminal.cursor.underline' },
  { id: 'bar', label: 'settings.terminal.cursor.bar' },
];

/** Reads a number input; returns null while the value is empty or out of range. */
function numberIn(input: HTMLInputElement, min: number, max: number): number | null {
  const value = input.valueAsNumber;
  return Number.isInteger(value) && value >= min && value <= max ? value : null;
}

export function TerminalSection() {
  onMount(() => void loadCliStatus());

  return (
    <Section title={t('settings.terminal')}>
      <Row label={t('settings.terminal.fontSize')} for="af-term-font">
        <Input
          id="af-term-font"
          type="number"
          min={TERMINAL_FONT_SIZE.min}
          max={TERMINAL_FONT_SIZE.max}
          value={settings.terminal.fontSize}
          onInput={(event) => {
            const size = numberIn(
              event.currentTarget,
              TERMINAL_FONT_SIZE.min,
              TERMINAL_FONT_SIZE.max,
            );
            if (size !== null) updateSettings((s) => (s.terminal.fontSize = size));
          }}
        />
      </Row>
      <Row label={t('settings.terminal.scrollback')} for="af-term-scrollback">
        <Input
          id="af-term-scrollback"
          type="number"
          min={0}
          max={TERMINAL_SCROLLBACK_MAX}
          step={500}
          value={settings.terminal.scrollback}
          onChange={(event) => {
            const lines = numberIn(event.currentTarget, 0, TERMINAL_SCROLLBACK_MAX);
            if (lines !== null) updateSettings((s) => (s.terminal.scrollback = lines));
          }}
        />
      </Row>
      <Row label={t('settings.terminal.cursorStyle')} for="af-term-cursor">
        <Select
          id="af-term-cursor"
          value={settings.terminal.cursorStyle}
          onChange={(event) => {
            const style = CURSORS.find((cursor) => cursor.id === event.currentTarget.value);
            if (style) updateSettings((s) => (s.terminal.cursorStyle = style.id));
          }}
        >
          <For each={CURSORS}>
            {(cursor) => <option value={cursor.id}>{t(cursor.label)}</option>}
          </For>
        </Select>
      </Row>
      <Row label={t('settings.terminal.autoStart')} for="af-term-autostart">
        <Toggle
          id="af-term-autostart"
          checked={settings.terminal.autoStartShells}
          onChange={(event) => {
            const on = event.currentTarget.checked;
            updateSettings((s) => (s.terminal.autoStartShells = on));
          }}
        />
      </Row>
      <p class="af-settings-hint">{t('settings.terminal.autoStartHint')}</p>

      <h3 class="af-settings-subtitle">{t('settings.terminal.clis')}</h3>
      <ul class="af-cli-list">
        <For each={cliStatus() ?? []}>
          {(cli) => (
            <li>
              <span class="af-cli-name">{kindLabel(cli.kind)}</span>
              <Show
                when={cli.found}
                fallback={<span class="af-cli-missing">{t('settings.terminal.cliMissing')}</span>}
              >
                <code class="af-cli-path" title={cli.path ?? ''}>
                  {cli.path}
                </code>
              </Show>
            </li>
          )}
        </For>
      </ul>
      <Button variant="secondary" size="sm" onClick={() => void loadCliStatus(true)}>
        {t('settings.terminal.rescan')}
      </Button>
    </Section>
  );
}
