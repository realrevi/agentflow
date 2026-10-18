//! The first-run flow (phase 11): whether it was finished or skipped, and whether the
//! tips card after it was dismissed. Settings > General can open the flow again.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OnboardingSettings {
    /// Finished or skipped; false shows the first-run flow on launch.
    pub done: bool,
    /// The small tips card shown after the flow was closed.
    pub tips_dismissed: bool,
}
