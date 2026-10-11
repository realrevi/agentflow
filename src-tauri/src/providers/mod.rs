//! AI provider APIs (phase 5): a `Provider` trait with two adapters (Anthropic Messages and
//! OpenAI-compatible Chat Completions), SSE streaming, and API keys in the OS keychain.

mod types;

pub use types::TokenUsage;
