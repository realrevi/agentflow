//! Anthropic prompt-cache breakpoints (`cache_control`). Limits from Anthropic's prompt
//! caching docs (checked 2026-10-17): at most 4 breakpoints per request, the system prompt
//! included; each breakpoint looks back at most 20 blocks for an earlier cache entry; the
//! default TTL is 5 minutes, refreshed by every hit; a prefix below the model's minimum
//! (512 to 4096 tokens) is simply not cached (no error, no write charge).
//!
//! Placement, most useful first: the end of the request (the next request, this one plus
//! the reply and a new prompt, reads it), the end of the history before the newest prompt
//! (a stable read point, one block from the previous request's breakpoint, so the 20-block
//! lookback always finds it) and the newest attachment. The adapter marks the system prompt.

use crate::providers::{ChatRequest, TurnBlock};

pub const MAX_BREAKPOINTS: usize = 4;

/// A block of `chat.turns[turn]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRef {
    pub turn: usize,
    pub block: usize,
}

/// Marks breakpoints on `chat` (which must have `caching` on); `attachments` are the
/// attachment blocks, oldest first.
pub fn place_breakpoints(chat: &mut ChatRequest, attachments: &[BlockRef]) {
    for block in chat.turns.iter_mut().flat_map(|t| t.blocks.iter_mut()) {
        block.cache = false;
    }
    let budget = MAX_BREAKPOINTS - usize::from(chat.caching && !chat.system.is_empty());
    let count = chat.turns.len();
    let mut wanted: Vec<BlockRef> = Vec::new();
    let candidates = [
        count.checked_sub(1).and_then(|turn| last_block(chat, turn)),
        count.checked_sub(2).and_then(|turn| last_block(chat, turn)),
        attachments
            .iter()
            .rev()
            .find(|b| is_cacheable(chat, **b))
            .copied(),
    ];
    for candidate in candidates.into_iter().flatten() {
        if wanted.len() < budget && !wanted.contains(&candidate) {
            wanted.push(candidate);
        }
    }
    for BlockRef { turn, block } in wanted {
        chat.turns[turn].blocks[block].cache = true;
    }
}

/// The last non-empty block of a turn (an empty block is never sent, so it cannot hold a
/// breakpoint).
fn last_block(chat: &ChatRequest, turn: usize) -> Option<BlockRef> {
    let blocks = &chat.turns.get(turn)?.blocks;
    (0..blocks.len())
        .rev()
        .map(|block| BlockRef { turn, block })
        .find(|b| is_cacheable(chat, *b))
}

/// Text, tool calls and tool results can carry a breakpoint; thinking and empty text not.
fn is_cacheable(chat: &ChatRequest, at: BlockRef) -> bool {
    chat.turns
        .get(at.turn)
        .and_then(|t| t.blocks.get(at.block))
        .is_some_and(TurnBlock::cacheable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ChatRole, ChatTurn};

    fn turn(role: ChatRole, texts: &[&str]) -> ChatTurn {
        ChatTurn {
            role,
            blocks: texts.iter().map(|t| TurnBlock::text(*t)).collect(),
        }
    }

    fn chat(system: &str, turns: Vec<ChatTurn>) -> ChatRequest {
        ChatRequest {
            model: "claude-opus-5-5".into(),
            system: system.into(),
            turns,
            max_tokens: 64,
            temperature: None,
            caching: true,
            tools: Vec::new(),
        }
    }

    fn marks(chat: &ChatRequest) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (t, turn) in chat.turns.iter().enumerate() {
            for (b, block) in turn.blocks.iter().enumerate() {
                if block.cache {
                    out.push((t, b));
                }
            }
        }
        out
    }

    #[test]
    fn marks_the_end_the_stable_prefix_and_the_newest_attachment() {
        use ChatRole::*;
        let mut request = chat(
            "Be brief.",
            vec![
                turn(User, &["File: a.rs\n...", "one"]),
                turn(Assistant, &["reply"]),
                turn(User, &["File: b.rs\n...", "File: c.rs\n...", "two"]),
            ],
        );
        let files = [
            BlockRef { turn: 0, block: 0 },
            BlockRef { turn: 2, block: 0 },
            BlockRef { turn: 2, block: 1 },
        ];
        place_breakpoints(&mut request, &files);
        assert_eq!(marks(&request), vec![(1, 0), (2, 1), (2, 2)]);
        // With the system prompt's breakpoint that is the API's maximum of 4.
        assert_eq!(marks(&request).len() + 1, MAX_BREAKPOINTS);
    }

    #[test]
    fn never_more_than_the_limit_and_never_on_empty_blocks() {
        use ChatRole::*;
        let mut request = chat(
            "",
            vec![
                turn(User, &["q"]),
                turn(Assistant, &["a", ""]),
                turn(User, &["x", ""]),
            ],
        );
        place_breakpoints(&mut request, &[BlockRef { turn: 2, block: 1 }]);
        assert_eq!(marks(&request), vec![(1, 0), (2, 0)]);

        // A first prompt: one breakpoint at its end, no duplicates.
        let mut first = chat("sys", vec![turn(User, &["File: a\n", "hi"])]);
        place_breakpoints(&mut first, &[BlockRef { turn: 0, block: 0 }]);
        assert_eq!(marks(&first), vec![(0, 0), (0, 1)]);
        assert!(marks(&first).len() < MAX_BREAKPOINTS);
    }
}
