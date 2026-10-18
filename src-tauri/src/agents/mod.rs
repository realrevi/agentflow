//! The agent installer (phase 8): one template format (template.rs, validate.rs), rendered
//! per tool (targets.rs) into files and managed sections (section.rs), planned and previewed
//! before writing (plan.rs, apply.rs), recorded in `.agentflow/installed.json` (registry.rs).
//! Detectors prefill answers from the workspace; installed AgentFlow agents are read from
//! `.agentflow/agents/` (profiles.rs). The community library (phase 10) feeds templates in.

mod apply;
mod binary;
mod customize;
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
pub use placeholders::escape as escape_placeholders;
pub use plan::{plan, Confirmations, InstallRequest, Plan};
pub use profiles::{get_profile, list_profiles, AgentProfile};
pub use registry::{hash_bytes, Installed, Registry};
pub use relpath::safe_relative;
pub use roots::Roots;
pub use starters::starters;
#[cfg(test)]
pub(crate) use targets::Scope;
pub use targets::{supports, Target};
pub use template::{
    ClaudeOptions, CodexOptions, CursorOptions, TargetOptions, Template, TemplateFile,
    TemplateKind, TemplateSource,
};
pub use validate::{validate, MAX_FILES, MAX_FILES_BYTES, MAX_TAGS};
