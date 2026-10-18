// Community agent library (phase 10), mirrored from src-tauri/src/library/model.rs and
// src-tauri/src/settings/library.rs (serde camelCase).

import type { AgentTemplate, InstallTarget, TemplateKind } from './installerTypes';

/** A public GitHub repository the user added (unverified), plus an optional folder. */
export interface CustomSource {
  url: string;
  path: string;
}

export interface LibrarySettings {
  customSources: CustomSource[];
  /** Refresh when the library opens and the last refresh is over a day old. */
  autoRefresh: boolean;
}

export type LibraryFormat =
  | 'claudeSubagent'
  | 'claudeSkill'
  | 'cursorRule'
  | 'codexAgent'
  | 'geminiCommand'
  | 'geminiAgent'
  | 'agentsMd';

/** Safety scan flag ids (src-tauri/src/library/safety.rs); shown as `library.flag.<id>`. */
export type LibraryFlagId =
  'downloadExec' | 'disablePermissions' | 'ignoreUser' | 'sendsData' | 'encodedBlob' | 'executable';

export interface LibraryFlag {
  id: LibraryFlagId;
  /** Relative to the item; empty = the main file. */
  file: string;
  excerpt: string;
}

export interface LibraryItemFile {
  path: string;
  size: number;
}

export interface LibraryItem {
  /** `<source id>:<path>`. */
  id: string;
  sourceId: string;
  sourceName: string;
  repo: string;
  verified: boolean;
  path: string;
  url: string;
  format: LibraryFormat;
  kind: TemplateKind;
  templateId: string;
  name: string;
  description: string;
  author: string;
  license: string;
  tags: string[];
  category: string | null;
  /** Every tool it installs for, and the ones the wizard preselects. */
  targets: InstallTarget[];
  native: InstallTarget[];
  tokens: number;
  flags: LibraryFlag[];
  files: LibraryItemFile[];
  /** Content hash: differs from an installed copy's `source.sha` = update available. */
  sha: string;
  commit: string;
  updatedAt: string | null;
  stars: number;
}

export interface LibraryUnsupported {
  sourceId: string;
  path: string;
  /** `library.unsupported.<reason>`. */
  reason: string;
  detail: string;
}

export interface LibrarySourceError {
  /** `rateLimited`, `notFound`, `network`, `invalid`, `http` or `tooLarge`. */
  kind: string;
  message: string;
  /** When GitHub's rate limit resets (ms since the epoch). */
  resetAt?: number;
}

export interface LibrarySource {
  id: string;
  name: string;
  repo: string;
  verified: boolean;
  license: string;
  stars: number;
  commit: string | null;
  updatedAt: string | null;
  checkedAt: number | null;
  imported: number;
  unsupported: number;
  error: LibrarySourceError | null;
}

export interface LibraryRate {
  limit: number;
  remaining: number;
  resetAt: number;
}

export interface LibraryCatalog {
  refreshedAt: number | null;
  sources: LibrarySource[];
  items: LibraryItem[];
  unsupported: LibraryUnsupported[];
  rate: LibraryRate | null;
  /** `remote`, `cache` or `bundled`. */
  registry: string;
}

export interface LibraryDetailFile {
  path: string;
  size: number;
  binary: boolean;
  cached: boolean;
  executable: boolean;
}

export interface LibraryDetail {
  item: LibraryItem;
  /** The main file as written upstream. */
  text: string;
  files: LibraryDetailFile[];
  flags: LibraryFlag[];
  template: AgentTemplate | null;
  /** `notCached`, `unsafePath` or `invalid` when there is no template. */
  problem: string | null;
}
