//! Token counts for the pre-send estimate. OpenAI models are counted exactly with tiktoken
//! (`o200k_base`, the encoding of GPT-4o, GPT-4.1, GPT-5 and the o-series) plus the chat
//! format overhead (4 per message, 3 to prime the reply). Other providers' tokenizers are not
//! available locally: the same o200k count is used as an approximation, marked "≈" (real
//! counts differ; Anthropic's newer tokenizer produces roughly 30% more tokens).

use serde::Serialize;

use crate::providers::{ChatRequest, ChatTurn, Part, TurnBlock};

const PER_MESSAGE: u64 = 4;
const REPLY_PRIMING: u64 = 3;

pub fn count(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    tiktoken_rs::o200k_base_singleton()
        .encode_with_special_tokens(text)
        .len() as u64
}

/// Exact for OpenAI's own models, approximate everywhere else.
pub fn is_exact(preset: &str, model: &str) -> bool {
    match preset {
        "openai" => true,
        "openrouter" => model.starts_with("openai/"),
        _ => false,
    }
}

pub fn turn_tokens(turn: &ChatTurn) -> u64 {
    turn.blocks.iter().map(block_tokens).sum::<u64>() + PER_MESSAGE
}

fn block_tokens(block: &TurnBlock) -> u64 {
    match &block.part {
        Part::Text { text } => count(text),
        Part::Reasoning(reasoning) => count(&reasoning.text),
        Part::ToolCall(call) => count(&call.name) + count(&call.input.to_string()),
        Part::ToolResult(result) => count(&result.content),
    }
}

/// Input tokens of a whole request (system prompt, history, attachments, new prompt).
pub fn request_tokens(chat: &ChatRequest) -> u64 {
    let system = if chat.system.is_empty() {
        0
    } else {
        count(&chat.system) + PER_MESSAGE
    };
    let tools: u64 = chat
        .tools
        .iter()
        .map(|tool| count(&tool.name) + count(&tool.description) + count(&tool.schema.to_string()))
        .sum();
    system + tools + chat.turns.iter().map(turn_tokens).sum::<u64>() + REPLY_PRIMING
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub tokens: u64,
    /// The count is an approximation ("≈").
    pub approximate: bool,
    /// USD for the input at the full input price; `None` without a known price.
    pub cost: Option<f64>,
    pub context_window: u64,
    /// Where the window came from: `endpoint`, `setting` or `default`.
    pub window_source: &'static str,
    /// `tokens / context_window`.
    pub share: f64,
    pub threshold: f64,
    /// Auto-compact is on and `share` reached `threshold`.
    pub should_compact: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_a_known_string_with_o200k() {
        // o200k_base: "hello world" is two tokens; "Hello, world!" is four.
        assert_eq!(count("hello world"), 2);
        assert_eq!(count("Hello, world!"), 4);
        assert_eq!(count(""), 0);
    }

    #[test]
    fn request_count_includes_format_overhead() {
        let chat = ChatRequest {
            model: "gpt-5".into(),
            system: "hello world".into(),
            turns: vec![ChatTurn::user("Hello, world!")],
            max_tokens: 16,
            temperature: None,
            caching: false,
            tools: Vec::new(),
        };
        assert_eq!(request_tokens(&chat), (2 + 4) + (4 + 4) + 3);
        assert!(is_exact("openai", "gpt-5"));
        assert!(is_exact("openrouter", "openai/gpt-5"));
        assert!(!is_exact("anthropic", "claude-opus-4-8"));
        assert!(!is_exact("deepseek", "deepseek-flash"));
    }
}
