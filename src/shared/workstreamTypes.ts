// Phase 9 IPC types: turn events from the CLI logs, workstream settings and the
// workstream log. Mirrors src-tauri/src/turns/events.rs, settings/workstreams.rs and
// workstreams/log.rs (serde camelCase).

export type HandoffMode = 'manual' | 'semiAuto' | 'auto';

export interface WorkstreamSettings {
  glow: boolean;
  /** `null` = the agent bold color. */
  glowColor: string | null;
  /** OS notification while the window is in the background. */
  notify: boolean;
  defaultMode: HandoffMode;
  maxRounds: number;
  reportCapKb: number;
  stageScale: number;
  stageAnimations: boolean;
  /** One automatic format reminder before pausing on a reply without a valid block. */
  autoRemind: boolean;
}

export type ActivityKind = 'think' | 'tool' | 'wait' | 'idle';
export type ToolAction = 'run' | 'edit' | 'read' | 'search' | 'web' | 'delegate' | 'plan' | 'tool';

export type TurnEvent =
  /** A log file was matched to the panel; `exact`: the log marks turn ends. */
  | { type: 'attached'; panel: string; exact: boolean }
  | {
      type: 'activity';
      panel: string;
      kind: ActivityKind;
      action: ToolAction | null;
      target: string | null;
    }
  | { type: 'draft'; panel: string; text: string }
  | { type: 'turnEnded'; panel: string; report: string; interrupted: boolean };

/** One handoff in the workstream log. */
export interface HandoffEntry {
  id: string;
  workstream: string;
  at: number;
  /** Panel ids. */
  from: string;
  to: string;
  fromName: string;
  toName: string;
  kind: 'prompt' | 'report' | 'setup';
  mode: HandoffMode;
  /** Bytes of the full text. */
  size: number;
  tokens: number | null;
  /** The full text (what was sent may be shorter: `truncated`). */
  text: string;
  truncated: boolean;
}
