//! Workstreams and the Agent Stage (phase 9): the attention glow, the optional OS
//! notification, the default handoff mode, the auto mode's round limit, the report cap and
//! the Stage's look.

use serde::{Deserialize, Serialize};

use crate::util::is_hex_color;

pub const MAX_ROUNDS_RANGE: (u32, u32) = (1, 100);
pub const REPORT_CAP_KB_RANGE: (u32, u32) = (1, 1024);
pub const STAGE_SCALES: [u8; 5] = [2, 3, 4, 5, 6];

/// How handoffs between a manager and its workers are sent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandoffMode {
    /// The user sends each handoff with a button (the text is editable).
    Manual,
    /// An editable preview card; one click sends.
    #[default]
    SemiAuto,
    /// Sent at once, until a stop condition.
    Auto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkstreamSettings {
    /// Panels that need the user glow (frame, tab, Stage character).
    pub glow: bool,
    /// `None` = the agent bold color.
    pub glow_color: Option<String>,
    /// An OS notification when a panel needs the user while the window is in the background.
    pub notify: bool,
    pub default_mode: HandoffMode,
    /// Auto mode stops after this many manager -> worker handoffs.
    pub max_rounds: u32,
    /// Reports longer than this (KB) reach the manager truncated; the log keeps them whole.
    pub report_cap_kb: u32,
    /// Stage character scale, one of `STAGE_SCALES`.
    pub stage_scale: u8,
    /// Packets, pulses and walking on the Stage (reduced motion turns them off too).
    pub stage_animations: bool,
}

impl Default for WorkstreamSettings {
    fn default() -> Self {
        Self {
            glow: true,
            glow_color: None,
            notify: false,
            default_mode: HandoffMode::SemiAuto,
            max_rounds: 10,
            report_cap_kb: 30,
            stage_scale: 4,
            stage_animations: true,
        }
    }
}

impl WorkstreamSettings {
    pub fn normalized(mut self) -> Self {
        let (min, max) = MAX_ROUNDS_RANGE;
        self.max_rounds = self.max_rounds.clamp(min, max);
        let (min, max) = REPORT_CAP_KB_RANGE;
        self.report_cap_kb = self.report_cap_kb.clamp(min, max);
        if !STAGE_SCALES.contains(&self.stage_scale) {
            self.stage_scale = Self::default().stage_scale;
        }
        self.glow_color = self.glow_color.take().filter(|c| is_hex_color(c));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_out_of_range_values() {
        let settings = WorkstreamSettings {
            max_rounds: 0,
            report_cap_kb: 99_999,
            stage_scale: 9,
            glow_color: Some("gold".into()),
            ..WorkstreamSettings::default()
        }
        .normalized();
        assert_eq!(settings.max_rounds, 1);
        assert_eq!(settings.report_cap_kb, 1024);
        assert_eq!(settings.stage_scale, 4);
        assert_eq!(settings.glow_color, None);
        let mode: HandoffMode = serde_json::from_str("\"semiAuto\"").expect("mode");
        assert_eq!(mode, HandoffMode::SemiAuto);
    }
}
