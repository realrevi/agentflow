//! Updates to a conversation's agent state (permission mode, tools allowed for the
//! conversation, the tools-off fallback, the environment snapshot of the system prompt).

use super::model::AgentState;
use super::store::ConversationStore;
use crate::error::AppResult;

impl ConversationStore {
    /// Changes the agent state and returns the new one.
    pub fn update_agent(
        &self,
        workspace: Option<&str>,
        id: &str,
        change: impl FnOnce(&mut AgentState),
    ) -> AppResult<AgentState> {
        self.modify(workspace, id, move |conversation| {
            change(&mut conversation.agent);
            Ok(())
        })
        .map(|conversation| conversation.agent)
    }
}
