// A tool card's expanded part: what the model asked for and what came back, monospace,
// long output capped until "show all".
import { createSignal, Show } from 'solid-js';
import type { ToolCall, ToolResult } from '../../../shared/types';
import { locale, t } from '../../i18n';
import { Button } from '../../../shared/ui/Button';

/** Characters shown before "show all". */
export const OUTPUT_PREVIEW = 3000;

/** The input as people read it: the command, the written content, or the arguments. */
export function inputText(call: ToolCall): string {
  const input = call.input;
  const str = (key: string) => (typeof input[key] === 'string' ? (input[key] as string) : '');
  switch (call.name) {
    case 'run_command':
      return str('command');
    case 'write_file':
      return `${str('path')}\n\n${str('content')}`;
    case 'edit_file':
      return `${str('path')}\n\n- ${str('old_string')}\n+ ${str('new_string')}`;
    default:
      return JSON.stringify(input, null, 2);
  }
}

function sizeText(chars: number): string {
  return new Intl.NumberFormat(locale(), {
    style: 'unit',
    unit: 'kilobyte',
    unitDisplay: 'narrow',
    maximumFractionDigits: 1,
  }).format(chars / 1000);
}

export function CappedText(props: { text: string }) {
  const [all, setAll] = createSignal(false);
  const long = () => props.text.length > OUTPUT_PREVIEW;
  return (
    <>
      <pre class="af-tool-pre">
        {all() || !long() ? props.text : `${props.text.slice(0, OUTPUT_PREVIEW)}…`}
      </pre>
      <Show when={long()}>
        <Button variant="ghost" size="sm" onClick={() => setAll((value) => !value)}>
          {all()
            ? t('agent.tool.showLess')
            : t('agent.tool.showMore', { size: sizeText(props.text.length) })}
        </Button>
      </Show>
    </>
  );
}

export function ToolDetails(props: { call: ToolCall; result: ToolResult | undefined }) {
  return (
    <div class="af-tool-details">
      <div class="af-tool-label">{t('agent.tool.input')}</div>
      <CappedText text={inputText(props.call)} />
      <div class="af-tool-label">{t('agent.tool.output')}</div>
      <Show when={props.result} fallback={<p class="af-tool-empty">{t('agent.tool.noOutput')}</p>}>
        {(result) => <CappedText text={result().content} />}
      </Show>
    </div>
  );
}
