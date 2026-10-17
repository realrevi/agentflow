// The model's reasoning as a collapsible "Thinking" section (see reasoning.ts).
import { createSignal, Show } from 'solid-js';
import { PixelIcon } from '../../../shared/ui/PixelIcon';
import { reasoningOpen, reasoningTitle, type ReasoningView } from './reasoning';
import { thinkIcon } from './toolIcons';

export function ReasoningBlock(props: { text: string; view: ReasoningView }) {
  const [toggled, setToggled] = createSignal<boolean | null>(null);
  const open = () => reasoningOpen(props.view, toggled());
  return (
    <div class="af-reasoning" classList={{ live: props.view.live && !props.view.answering }}>
      <button
        type="button"
        class="af-reasoning-head"
        aria-expanded={open()}
        onClick={() => setToggled(!open())}
      >
        <PixelIcon art={thinkIcon} class="af-tool-icon" />
        <span>{reasoningTitle(props.view)}</span>
      </button>
      <Show when={open()}>
        <pre class="af-reasoning-text">{props.text}</pre>
      </Show>
    </div>
  );
}
