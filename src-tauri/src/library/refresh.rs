//! Refreshing one source: repository (stars, default branch, license) and branch head with
//! ETags, the recursive tree only when the commit changed, then the missing item files from
//! raw.githubusercontent.com, a few at a time, within the size caps. A used-up rate limit
//! ends the whole refresh; whatever is cached stays usable.

use std::sync::Arc;

use serde_json::Value;
use tokio::task::JoinSet;

use super::build::MAX_FILE;
use super::cache::{Cache, Snapshot, TreeFile};
use super::github::{Fetched, GhError, GitHub};
use super::model::Source;
use super::select;
use crate::util::now_ms;

/// Raw file downloads in flight per source.
const PARALLEL: usize = 8;
/// Bytes a source may add to the cache in one refresh.
const MAX_SOURCE_BYTES: u64 = 24 * 1024 * 1024;
/// Tree entries kept (GitHub's own cap is 100 000).
const MAX_TREE: usize = 100_000;

fn text(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current.as_str().map(str::to_owned)
}

pub async fn source(github: &Arc<GitHub>, cache: &Cache, source: &Source) -> Result<(), GhError> {
    let mut snap = cache
        .snapshot(&source.id)
        .filter(|s| s.repo == source.repo)
        .unwrap_or_else(|| Snapshot {
            repo: source.repo.clone(),
            ..Snapshot::default()
        });
    let repo = &source.repo;
    if let Fetched::Fresh { body, etag } = github
        .api(&format!("repos/{repo}"), snap.repo_etag.as_deref())
        .await?
    {
        snap.stars = body
            .get("stargazers_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        snap.default_branch = text(&body, &["default_branch"]).unwrap_or_else(|| "main".into());
        snap.license = text(&body, &["license", "spdx_id"]).filter(|l| l != "NOASSERTION");
        snap.repo_etag = etag;
    }
    let branch = source
        .branch
        .clone()
        .unwrap_or_else(|| snap.default_branch.clone());
    if branch.is_empty() {
        return Err(GhError::Invalid(
            "the repository has no default branch".into(),
        ));
    }
    if snap.branch != branch {
        snap.branch_etag = None;
    }
    let path = format!("repos/{repo}/branches/{branch}");
    if let Fetched::Fresh { body, etag } = github.api(&path, snap.branch_etag.as_deref()).await? {
        let commit = text(&body, &["commit", "sha"]).ok_or(GhError::Invalid("no commit".into()))?;
        let tree = text(&body, &["commit", "commit", "tree", "sha"])
            .ok_or(GhError::Invalid("no tree".into()))?;
        if commit != snap.commit || branch != snap.branch {
            snap.files.clear();
        }
        snap.commit = commit;
        snap.tree = tree;
        snap.branch = branch;
        snap.committed_at = text(&body, &["commit", "commit", "committer", "date"]);
        snap.branch_etag = etag;
    }
    if snap.files.is_empty() {
        let path = format!("repos/{repo}/git/trees/{}?recursive=1", snap.tree);
        if let Fetched::Fresh { body, .. } = github.api(&path, None).await? {
            snap.truncated = body
                .get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            snap.files = tree_files(&body);
        }
    }
    // Saved before the downloads, so a failed download keeps the new tree.
    let fetched = fetch_missing(github, cache, source, &snap).await;
    snap.checked_at = now_ms();
    cache
        .put_snapshot(&source.id, &snap)
        .map_err(|err| GhError::Invalid(format!("could not write the cache: {err}")))?;
    fetched
}

fn tree_files(body: &Value) -> Vec<TreeFile> {
    let Some(entries) = body.get("tree").and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|e| e.get("type").and_then(Value::as_str) == Some("blob"))
        .filter_map(|e| {
            Some(TreeFile {
                path: e.get("path")?.as_str()?.to_owned(),
                sha: e.get("sha")?.as_str()?.to_owned(),
                size: e.get("size").and_then(Value::as_u64).unwrap_or(0),
            })
        })
        .take(MAX_TREE)
        .collect()
}

async fn fetch_missing(
    github: &Arc<GitHub>,
    cache: &Cache,
    source: &Source,
    snap: &Snapshot,
) -> Result<(), GhError> {
    let mut budget = MAX_SOURCE_BYTES;
    let mut queue: Vec<TreeFile> = Vec::new();
    for file in select::needed(source, snap, cache) {
        if file.size <= budget {
            budget -= file.size;
            queue.push(file.clone());
        }
    }
    fetch_files(github, cache, &source.repo, &snap.commit, queue).await
}

/// Downloads `files` at `commit` into the blob cache, `PARALLEL` at a time. The first
/// rate limit or network error stops the rest (and is returned).
pub async fn fetch_files(
    github: &Arc<GitHub>,
    cache: &Cache,
    repo: &str,
    commit: &str,
    files: Vec<TreeFile>,
) -> Result<(), GhError> {
    let mut pending = files.into_iter();
    let mut running = JoinSet::new();
    let mut failure = None;
    loop {
        while failure.is_none() && running.len() < PARALLEL {
            let Some(file) = pending.next() else { break };
            let (github, cache) = (github.clone(), cache.clone());
            let (repo, commit) = (repo.to_owned(), commit.to_owned());
            running.spawn(async move {
                let bytes = github
                    .raw(&repo, &commit, &file.path, MAX_FILE as usize)
                    .await;
                match bytes {
                    Ok(bytes) => cache.put_blob(&file.sha, &bytes).map_err(|err| {
                        GhError::Invalid(format!("could not write the cache: {err}"))
                    }),
                    // A file that vanished or grew is just not imported.
                    Err(GhError::NotFound | GhError::TooLarge) => Ok(()),
                    Err(err) => Err(err),
                }
            });
        }
        let Some(done) = running.join_next().await else {
            break;
        };
        match done {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                failure.get_or_insert(err);
            }
            Err(err) => {
                failure.get_or_insert(GhError::Invalid(format!("download task failed: {err}")));
            }
        }
    }
    failure.map_or(Ok(()), Err)
}
