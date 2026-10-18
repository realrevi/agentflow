//! Which tree files of a source are items (its globs, or a guess per file for custom
//! sources), a skill's other files, and which contents a refresh must download.

use std::collections::{HashMap, HashSet};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::build::{MAX_FILE, MAX_ITEMS};
use super::cache::{Cache, Snapshot, TreeFile};
use super::import::{guess, parent, LICENSE_FILES};
use super::model::{Format, Source, Unsupported};

/// Hidden folders a custom source may still hold items in.
const TOOL_FOLDERS: [&str; 5] = [".claude", ".gemini", ".cursor", ".codex", ".agents"];
pub struct Candidate {
    pub path: String,
    pub sha: String,
    pub size: u64,
    pub format: Option<Format>,
    pub entry: usize,
}

fn globs(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = globset::GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .or_else(|_| Glob::new(pattern))
            .ok()?;
        builder.add(glob);
    }
    builder.build().ok()
}

/// The tree files that are items of `source`, and a "too many" entry if it has too many.
pub fn candidates(source: &Source, files: &[TreeFile]) -> (Vec<Candidate>, Vec<Unsupported>) {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (index, entry) in source.entries.iter().enumerate() {
        let (Some(include), Some(exclude)) = (globs(&entry.include), globs(&entry.exclude)) else {
            continue;
        };
        let guessing = entry.format.is_none();
        let skill_dirs: HashSet<&str> = if guessing {
            files
                .iter()
                .filter(|f| f.path.ends_with("/SKILL.md") || f.path == "SKILL.md")
                .map(|f| parent(&f.path))
                .collect()
        } else {
            HashSet::new()
        };
        for file in files {
            if !include.is_match(&file.path)
                || exclude.is_match(&file.path)
                || !seen.insert(&file.path)
            {
                continue;
            }
            let format = if guessing {
                let hidden = file.path.split('/').any(|part| {
                    part.starts_with('.') && part.len() > 1 && !TOOL_FOLDERS.contains(&part)
                });
                let in_skill = skill_dirs.iter().any(|dir| {
                    file.path.starts_with(&format!("{dir}/")) && !file.path.ends_with("/SKILL.md")
                }) || (skill_dirs.contains("") && file.path != "SKILL.md");
                if hidden || in_skill || guess(&file.path, None).is_none() {
                    continue;
                }
                None
            } else {
                entry.format
            };
            out.push(Candidate {
                path: file.path.clone(),
                sha: file.sha.clone(),
                size: file.size,
                format,
                entry: index,
            });
        }
    }
    let mut unsupported = Vec::new();
    if out.len() > MAX_ITEMS {
        unsupported.push(Unsupported {
            source_id: source.id.clone(),
            path: String::new(),
            reason: "tooMany".into(),
            detail: format!("{} files; the first {MAX_ITEMS} are imported", out.len()),
        });
        out.truncate(MAX_ITEMS);
    }
    (out, unsupported)
}

/// A skill's other files (relative to its folder), from the tree.
pub fn skill_files<'a>(files: &'a [TreeFile], main: &str) -> Vec<&'a TreeFile> {
    let dir = parent(main);
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    files
        .iter()
        .filter(|f| f.path.starts_with(&prefix) && f.path != main)
        .collect()
}

/// Which blobs a refresh must have: main files, and per-item license files.
pub fn needed<'a>(source: &Source, snapshot: &'a Snapshot, cache: &Cache) -> Vec<&'a TreeFile> {
    let (candidates, _) = candidates(source, &snapshot.files);
    let by_path: HashMap<&str, &TreeFile> = snapshot
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut out = Vec::new();
    for c in &candidates {
        let mut wanted = vec![c.path.clone()];
        if source.license == "per-item" {
            let dir = parent(&c.path);
            wanted.extend(LICENSE_FILES.iter().map(|name| join(dir, name)));
        }
        for path in wanted {
            if let Some(file) = by_path.get(path.as_str()) {
                if file.size <= MAX_FILE && !cache.has_blob(&file.sha) {
                    out.push(*file);
                }
            }
        }
    }
    out
}

pub fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}
