// Up/Down through earlier prompts, like a shell. Pure, so the prompt bar stays simple.

export interface Recall {
  /** Index into the history (oldest first), or null while editing a fresh draft. */
  index: number | null;
  draft: string;
}

export const FRESH: Recall = { index: null, draft: '' };

/** Older prompt; returns the text to show and the new recall position. */
export function older(
  history: readonly string[],
  recall: Recall,
  current: string,
): { text: string; recall: Recall } | null {
  if (history.length === 0) return null;
  const index = recall.index === null ? history.length - 1 : Math.max(0, recall.index - 1);
  const draft = recall.index === null ? current : recall.draft;
  return { text: history[index] ?? '', recall: { index, draft } };
}

/** Newer prompt, or back to the draft after the newest one. */
export function newer(
  history: readonly string[],
  recall: Recall,
): { text: string; recall: Recall } | null {
  if (recall.index === null) return null;
  const index = recall.index + 1;
  if (index >= history.length) return { text: recall.draft, recall: FRESH };
  return { text: history[index] ?? '', recall: { index, draft: recall.draft } };
}
