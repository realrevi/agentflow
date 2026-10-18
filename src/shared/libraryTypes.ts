// Community agent library (phase 10), mirrored from src-tauri/src/library/model.rs and
// src-tauri/src/settings/library.rs (serde camelCase).

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
