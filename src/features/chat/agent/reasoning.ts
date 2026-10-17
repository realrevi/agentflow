// When the "Thinking" section is open and what its header says. Open while the model
// thinks; collapses on its own when the answer starts; a click overrides both.
import { t } from '../../i18n';
import { durationText } from './toolText';

export interface ReasoningView {
  /** Reasoning still streams in. */
  live: boolean;
  /** The answer (text or a tool call) started. */
  answering: boolean;
  durationMs: number | undefined;
}

/** `toggled`: the user's choice (`null` = none yet). */
export function reasoningOpen(view: ReasoningView, toggled: boolean | null): boolean {
  return toggled ?? (view.live && !view.answering);
}

export function reasoningTitle(view: ReasoningView): string {
  if (view.live && !view.answering) return t('agent.reasoning.live');
  return view.durationMs !== undefined
    ? t('agent.reasoning.done', { duration: durationText(view.durationMs) })
    : t('agent.reasoning.title');
}
