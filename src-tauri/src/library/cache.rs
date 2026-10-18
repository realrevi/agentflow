//! The library cache in the app data dir (`library/`): file contents by git blob sha
//! (`blobs/ab/abcdef...`), one snapshot per source (commit, tree, ETags), the registry
//! as last fetched, and the built catalog. Everything works offline from here.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::Catalog;
use crate::util::write_atomic;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeFile {
    pub path: String,
    pub sha: String,
    pub size: u64,
}

/// What a refresh learned about one source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Snapshot {
    pub repo: String,
    pub default_branch: String,
    /// The repository's SPDX license id as GitHub reports it.
    pub license: Option<String>,
    pub branch: String,
    pub commit: String,
    pub tree: String,
    pub committed_at: Option<String>,
    pub stars: u64,
    pub repo_etag: Option<String>,
    pub branch_etag: Option<String>,
    pub checked_at: u64,
    /// GitHub cut the tree short (over 100 000 entries or 7 MB).
    pub truncated: bool,
    /// Every file (blob) of the tree.
    pub files: Vec<TreeFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CachedRegistry {
    pub etag: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

fn is_sha(sha: &str) -> bool {
    (7..=64).contains(&sha.len()) && sha.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Source ids become file names: only `a-z0-9-` (registry and custom ids are slugs).
fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    write_atomic(path, &json)
}

impl Cache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn blob_path(&self, sha: &str) -> Option<PathBuf> {
        is_sha(sha).then(|| self.dir.join("blobs").join(&sha[..2]).join(sha))
    }

    pub fn has_blob(&self, sha: &str) -> bool {
        self.blob_path(sha).is_some_and(|p| p.is_file())
    }

    pub fn blob(&self, sha: &str) -> Option<Vec<u8>> {
        std::fs::read(self.blob_path(sha)?).ok()
    }

    pub fn put_blob(&self, sha: &str, bytes: &[u8]) -> std::io::Result<()> {
        match self.blob_path(sha) {
            Some(path) => write_atomic(&path, bytes),
            None => Err(std::io::Error::other("bad blob sha")),
        }
    }

    pub fn snapshot(&self, id: &str) -> Option<Snapshot> {
        is_id(id).then(|| read_json(&self.dir.join("snapshots").join(format!("{id}.json"))))?
    }

    pub fn put_snapshot(&self, id: &str, snapshot: &Snapshot) -> std::io::Result<()> {
        if !is_id(id) {
            return Err(std::io::Error::other("bad source id"));
        }
        write_json(
            &self.dir.join("snapshots").join(format!("{id}.json")),
            snapshot,
        )
    }

    pub fn registry(&self) -> Option<CachedRegistry> {
        read_json(&self.dir.join("registry.json"))
    }

    pub fn put_registry(&self, registry: &CachedRegistry) -> std::io::Result<()> {
        write_json(&self.dir.join("registry.json"), registry)
    }

    pub fn catalog(&self) -> Option<Catalog> {
        read_json(&self.dir.join("catalog.json"))
    }

    pub fn put_catalog(&self, catalog: &Catalog) -> std::io::Result<()> {
        write_json(&self.dir.join("catalog.json"), catalog)
    }

    /// Removes blobs no snapshot refers to any more, and snapshots of removed sources.
    pub fn prune(&self, keep_sources: &[String]) {
        let mut keep = HashSet::new();
        let snapshots = self.dir.join("snapshots");
        for entry in std::fs::read_dir(&snapshots)
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json").unwrap_or(&name).to_owned();
            if !keep_sources.contains(&id) {
                let _ = std::fs::remove_file(entry.path());
                continue;
            }
            if let Some(snapshot) = self.snapshot(&id) {
                keep.extend(snapshot.files.into_iter().map(|f| f.sha));
            }
        }
        let blobs = self.dir.join("blobs");
        for shard in std::fs::read_dir(&blobs).into_iter().flatten().flatten() {
            for blob in std::fs::read_dir(shard.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let sha = blob.file_name().to_string_lossy().into_owned();
                if !keep.contains(&sha) {
                    let _ = std::fs::remove_file(blob.path());
                }
            }
        }
    }
}
