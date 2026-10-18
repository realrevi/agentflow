// What the layout lends the Stage (it cannot import the layout or panel features).
import type { CharacterRef } from '../characters/types';

export interface StagePanelInfo {
  id: string;
  title: string;
  /** Agent CLI, custom command or API chat. */
  agent: boolean;
  character: CharacterRef | null;
}

export interface StageHooks {
  /** Reactive: the open panels. */
  panels(): readonly StagePanelInfo[];
  focus(panelId: string): void;
  openTimeline(workstream: string, pair: [string, string]): void;
  newWorkstream(): void;
  move(target: 'left' | 'center'): void;
  isLeft(): boolean;
  /** Where a panel's prompt history is stored. */
  history(panelId: string): { workspace: string | null; key: string } | null;
}
