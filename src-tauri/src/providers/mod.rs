//! AI provider APIs (phase 5): a `Provider` trait with two adapters (Anthropic Messages and
//! OpenAI-compatible Chat Completions), SSE streaming, cancellation, and API keys that live
//! only in the OS keychain.

mod anthropic;
mod anthropic_request;
mod anthropic_stream;
mod cancel;
mod error;
mod http;
mod keys;
mod openai;
mod openai_request;
mod openai_stream;
mod provider;
mod sse;
#[cfg(test)]
pub(crate) mod tests;
mod types;
mod url;

pub use cancel::{cancel_pair, CancelHandle, CancelToken};
pub use error::{redact, ErrorKind, ProviderError};
pub use http::Http;
#[cfg(test)]
pub use keys::MemoryStore;
pub use keys::{KeyStatus, KeyVault, OsKeychain, SecretStore};
pub use provider::{build_provider, Provider};
pub use types::{
    ChatRequest, ChatRole, ChatTurn, EventSink, ModelInfo, Part, Reasoning, StreamEnd, StreamEvent,
    TokenUsage, ToolCall, ToolResult, ToolSpec, TurnBlock,
};
