//! Tauri commands. Keep them thin: validate input, delegate to a module, return `AppResult`.
//! Every command has a typed wrapper in `src/shared/ipc.ts`.

pub mod agent;
pub mod chat;
pub mod conversations;
pub mod fs;
pub mod installer;
pub mod prompts;
pub mod providers;
pub mod pty;
pub mod settings;
pub mod themes;
pub mod usage;
