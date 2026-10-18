//! From a cached snapshot and file contents to a source's cards: each item imported, scanned
//! and counted, or listed as unsupported with the reason. Pure apart from reading the cache.

use std::collections::{HashMap, HashSet};

use super::cache::{Cache, Snapshot, TreeFile};
use super::import::{self, guess, parent, Input, LICENSE_FILES};
use super::model::{Format, Item, ItemFile, Source, Unsupported};
use super::safety;
use super::select::{candidates, join, skill_files, Candidate};
use crate::agents::{self, Target};
use crate::tokens::count_tokens;

/// Files above this are not fetched or imported.
pub const MAX_FILE: u64 = 512 * 1024;
/// Items per source; more are listed as one "too many" entry.
pub const MAX_ITEMS: usize = 1000;
const ALL_TARGETS: [Target; 5] = [
    Target::ClaudeCode,
    Target::Codex,
    Target::Gemini,
    Target::Cursor,
    Target::Agentflow,
];

/// Content hash of an item: its main blob, or (skills) every blob of its folder.
fn item_sha(candidate: &Candidate, folder: &[&TreeFile], skill: bool) -> String {
    if !skill {
        return candidate.sha.clone();
    }
    let mut parts: Vec<String> = folder
        .iter()
        .map(|f| format!("{}:{}", f.path, f.sha))
        .collect();
    parts.push(format!("{}:{}", candidate.path, candidate.sha));
    parts.sort();
    agents::hash_bytes(parts.join("\n").as_bytes())
        .chars()
        .take(40)
        .collect()
}

pub struct Built {
    pub items: Vec<Item>,
    pub unsupported: Vec<Unsupported>,
}

/// One source's items from its snapshot (missing contents are "not fetched").
pub fn source_items(source: &Source, snapshot: &Snapshot, cache: &Cache) -> Built {
    let (candidates, mut unsupported) = candidates(source, &snapshot.files);
    let by_path: HashMap<&str, &TreeFile> = snapshot
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();
    let license = if source.license.is_empty() {
        snapshot.license.clone().unwrap_or_else(|| "unknown".into())
    } else {
        source.license.clone()
    };
    let mut items: Vec<Item> = Vec::new();
    let mut seen = HashSet::new();
    let mut skip = |path: &str, reason: &str, detail: String| {
        unsupported.push(Unsupported {
            source_id: source.id.clone(),
            path: path.to_owned(),
            reason: reason.into(),
            detail,
        });
    };
    for c in &candidates {
        if c.size > MAX_FILE {
            skip(&c.path, "tooLarge", format!("{} KB", c.size / 1024));
            continue;
        }
        let Some(bytes) = cache.blob(&c.sha) else {
            skip(&c.path, "notFetched", String::new());
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            skip(&c.path, "notText", String::new());
            continue;
        };
        let Some(format) = c.format.or_else(|| guess(&c.path, Some(&text))) else {
            continue;
        };
        let skill = format == Format::ClaudeSkill;
        let folder = if skill {
            skill_files(&snapshot.files, &c.path)
        } else {
            Vec::new()
        };
        let item_license = LICENSE_FILES
            .iter()
            .filter_map(|name| by_path.get(join(parent(&c.path), name).as_str()))
            .find_map(|f| cache.blob(&f.sha))
            .and_then(|b| String::from_utf8(b).ok());
        let sha = item_sha(c, &folder, skill);
        let input = Input {
            format,
            path: &c.path,
            text: &text,
            repo: &source.repo,
            commit: &snapshot.commit,
            sha: &sha,
            license: &license,
            item_license: item_license.as_deref().filter(|_| license == "per-item"),
            verified: source.verified,
        };
        let imported = match import::import(&input) {
            Ok(imported) => imported,
            Err(why) => {
                skip(&c.path, why.reason, why.detail);
                continue;
            }
        };
        // The same file in several places (wshobson's plugins share agents) appears once.
        if !seen.insert((imported.template.id.clone(), c.sha.clone())) {
            continue;
        }
        let total: u64 = folder.iter().map(|f| f.size).sum();
        if folder.len() > agents::MAX_FILES || total > agents::MAX_FILES_BYTES as u64 {
            skip(
                &c.path,
                "tooLarge",
                format!("{} files, {} KB", folder.len(), total / 1024),
            );
            continue;
        }
        let dir = parent(&c.path);
        let files: Vec<ItemFile> = folder
            .iter()
            .map(|f| ItemFile {
                path: f.path[dir.len()..].trim_start_matches('/').to_owned(),
                size: f.size,
            })
            .collect();
        let template = &imported.template;
        let targets: Vec<Target> = ALL_TARGETS
            .into_iter()
            .filter(|t| agents::supports(template, *t))
            .collect();
        let preferred = source.entries[c.entry]
            .targets
            .clone()
            .unwrap_or(imported.native);
        let native: Vec<Target> = preferred
            .into_iter()
            .filter(|t| targets.contains(t))
            .collect();
        let mut flags = safety::scan_text("", &text);
        flags.extend(safety::scan_files(files.iter().map(|f| f.path.as_str())));
        let url = if skill {
            format!(
                "https://github.com/{}/tree/{}/{dir}",
                source.repo, snapshot.commit
            )
        } else {
            format!(
                "https://github.com/{}/blob/{}/{}",
                source.repo, snapshot.commit, c.path
            )
        };
        items.push(Item {
            id: format!("{}:{}", source.id, c.path),
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            repo: source.repo.clone(),
            verified: source.verified,
            path: c.path.clone(),
            url,
            format,
            kind: template.kind,
            template_id: template.id.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            author: template.author.clone(),
            license: template.license.clone(),
            tags: template.tags.clone(),
            category: imported.category.clone(),
            targets,
            native,
            tokens: count_tokens(&template.instructions),
            flags,
            files,
            sha,
            commit: snapshot.commit.clone(),
            updated_at: snapshot.committed_at.clone(),
            stars: snapshot.stars,
        });
    }
    Built { items, unsupported }
}
