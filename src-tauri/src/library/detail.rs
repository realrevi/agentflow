//! One item in full: its original text, every file (a skill's other files are downloaded
//! on first view, then cached), the safety scan over all of them, and the template the
//! installer gets (the phase 8 wizard previews every file before anything is written).

use std::sync::Arc;

use serde::Serialize;

use super::cache::Cache;
use super::github::GitHub;
use super::import::{self, parent, Input};
use super::model::{Item, ItemFile};
use super::refresh::fetch_files;
use super::safety::{self, Flag};
use super::select::skill_files;
use crate::agents::{self, Template, TemplateFile};
use crate::error::{AppError, AppResult};
use crate::util::base64;

/// Extensions that are binary even when they happen to be valid UTF-8.
const BINARY: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "pdf", "zip", "gz", "tgz", "woff", "woff2",
    "ttf", "otf", "exe", "dll", "so", "dylib", "jar", "bin", "wasm",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailFile {
    pub path: String,
    pub size: u64,
    pub binary: bool,
    pub cached: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDetail {
    pub item: Item,
    /// The main file as written upstream (frontmatter included).
    pub text: String,
    pub files: Vec<DetailFile>,
    pub flags: Vec<Flag>,
    /// For the wizard; `None` while files are missing (offline) or a path is unsafe.
    pub template: Option<Template>,
    /// Why there is no template (`notCached`, `unsafePath`, `invalid`).
    pub problem: Option<String>,
}

fn is_binary(path: &str, bytes: &[u8]) -> bool {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    ext.is_some_and(|e| BINARY.contains(&e.as_str())) || std::str::from_utf8(bytes).is_err()
}

pub async fn detail(
    github: Option<Arc<GitHub>>,
    cache: &Cache,
    item: Item,
) -> AppResult<ItemDetail> {
    let snapshot = cache
        .snapshot(&item.source_id)
        .ok_or_else(|| AppError::NotFound("this item is not cached; refresh the library".into()))?;
    let main = snapshot
        .files
        .iter()
        .find(|f| f.path == item.path)
        .and_then(|f| cache.blob(&f.sha))
        .ok_or_else(|| AppError::NotFound("this item is not cached; refresh the library".into()))?;
    let text = String::from_utf8(main)
        .map_err(|_| AppError::InvalidInput("the item is not a text file".into()))?;
    let folder = if item.files.is_empty() {
        Vec::new()
    } else {
        skill_files(&snapshot.files, &item.path)
    };
    let missing: Vec<_> = folder
        .iter()
        .filter(|f| !cache.has_blob(&f.sha))
        .map(|f| (*f).clone())
        .collect();
    if let (Some(github), false) = (github, missing.is_empty()) {
        if let Err(err) = fetch_files(&github, cache, &item.repo, &snapshot.commit, missing).await {
            tracing::info!("skill files not fetched: {}", err.message());
        }
    }
    let dir = parent(&item.path);
    let mut flags = safety::scan_text("", &text);
    let mut files = Vec::new();
    let mut template_files = Vec::new();
    let mut problem = None;
    for file in &folder {
        let rel = file.path[dir.len()..].trim_start_matches('/').to_owned();
        let bytes = cache.blob(&file.sha);
        let binary = bytes.as_deref().is_some_and(|b| is_binary(&rel, b));
        files.push(DetailFile {
            path: rel.clone(),
            size: file.size,
            binary,
            cached: bytes.is_some(),
            executable: safety::is_executable(&rel),
        });
        let Some(bytes) = bytes else {
            problem.get_or_insert("notCached");
            continue;
        };
        let Ok(safe) = agents::safe_relative(&rel) else {
            problem = Some("unsafePath");
            continue;
        };
        let content = if binary {
            base64::encode(&bytes)
        } else {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            flags.extend(safety::scan_text(&rel, &text));
            text
        };
        template_files.push(TemplateFile {
            path: safe,
            content,
            binary,
        });
    }
    flags.extend(safety::scan_files(files.iter().map(|f| f.path.as_str())));
    let template = match problem {
        Some(_) => None,
        None => build_template(&item, &text, template_files)
            .map_err(|e| problem = Some(e))
            .ok(),
    };
    Ok(ItemDetail {
        item: Item {
            files: files
                .iter()
                .map(|f| ItemFile {
                    path: f.path.clone(),
                    size: f.size,
                })
                .collect(),
            ..item
        },
        text,
        files,
        flags,
        template,
        problem: problem.map(str::to_owned),
    })
}

fn build_template(
    item: &Item,
    text: &str,
    files: Vec<TemplateFile>,
) -> Result<Template, &'static str> {
    let input = Input {
        format: item.format,
        path: &item.path,
        text,
        repo: &item.repo,
        commit: &item.commit,
        sha: &item.sha,
        license: &item.license,
        item_license: None,
        verified: item.verified,
    };
    let mut template = import::import(&input).map_err(|_| "invalid")?.template;
    template.files = files;
    if agents::validate(&template).is_empty() {
        Ok(template)
    } else {
        Err("invalid")
    }
}
