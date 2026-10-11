// Shown over a stopped terminal: what it runs, the last error, and a Start button.
import { Show } from 'solid-js';
import { agentsIcon } from '../../shared/ui/icons';
import { PixelIcon } from '../../shared/ui/PixelIcon';
import { t } from '../i18n';
import { isAgentKind, terminalTitle, type TerminalConfig } from './config';

export function StartOverlay(props: {
  config: TerminalConfig;
  error: string | null;
  onStart: () => void;
}) {
  const commandLine = () =>
    props.config.kind === 'custom' ? [props.config.command, ...props.config.args].join(' ') : '';
  return (
    <div class="af-terminal-overlay">
      <PixelIcon art={agentsIcon} scale={3} />
      <h2 class="af-pixel-title">{terminalTitle(props.config)}</h2>
      <Show when={commandLine()}>
        <code class="af-terminal-command">{commandLine()}</code>
      </Show>
      <Show when={props.error}>{(message) => <p class="af-terminal-error">{message()}</p>}</Show>
      <button type="button" class="af-button" onClick={() => props.onStart()}>
        {t('terminal.start')}
      </button>
      <Show when={isAgentKind(props.config.kind)}>
        <p class="af-terminal-hint">{t('terminal.agentHint')}</p>
      </Show>
    </div>
  );
}
