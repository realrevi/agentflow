//! The agent installer (phase 8): one template format (template.rs, validate.rs), rendered
//! per tool (targets.rs) into files and managed sections (section.rs), planned and previewed
//! before writing (plan.rs, apply.rs), recorded in `.agentflow/installed.json` (registry.rs).
//! Detectors prefill answers from the workspace; installed AgentFlow agents are read from
//! `.agentflow/agents/` (profiles.rs). The community library (phase 10) feeds templates in.

mod apply;
mod binary;
mod detect;
mod detect_style;
mod generate;
mod placeholders;
mod plan;
mod profiles;
mod registry;
mod relpath;
mod roots;
mod section;
mod skills;
mod starters;
mod targets;
mod template;
mod text;
mod validate;

#[cfg(test)]
mod tests;

pub use apply::{install, uninstall, uninstall_plan};
pub use detect::{detect, Detection};
pub use generate::{generate, GenerateRequest, GenerateResult};
pub use plan::{plan, Confirmations, InstallRequest, Plan};
pub use profiles::{get_profile, list_profiles, AgentProfile};
pub use registry::{Installed, Registry};
pub use roots::Roots;
pub use starters::starters;
pub use template::Template;
pub use validate::validate;
