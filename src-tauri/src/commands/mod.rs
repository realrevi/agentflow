//! Tauri commands. Keep them thin: validate input, delegate to a module, return `AppResult`.
//! Every command has a typed wrapper in `src/shared/ipc.ts`.

pub mod fs;
pub mod pty;
pub mod settings;
pub mod themes;
