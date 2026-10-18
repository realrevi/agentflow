//! The source list: AgentFlow's registry (`registry/sources.json`, the newest copy from
//! GitHub, else the cached one, else the copy built into the app) plus the user's custom
//! sources from settings, which are always unverified.

use serde::Deserialize;

use super::cache::{Cache, CachedRegistry};
use super::github::{Fetched, GitHub};
use super::import::slug;
use super::model::{Entry, Source};
use crate::settings::CustomSource;

const BUNDLED: &str = include_str!("../../../registry/sources.json");
const REGISTRY_VERSION: u32 = 1;
const MAX_SOURCES: usize = 40;

#[derive(Deserialize)]
struct RegistryFile {
    version: u32,
    sources: Vec<Source>,
}

/// The registry's sources, all verified; `None` if the text is not a valid registry.
pub fn parse_registry(text: &str) -> Option<Vec<Source>> {
    let file: RegistryFile = serde_json::from_str(text).ok()?;
    if file.version != REGISTRY_VERSION
        || file.sources.is_empty()
        || file.sources.len() > MAX_SOURCES
    {
        return None;
    }
    let valid = file.sources.iter().all(|s| {
        slug(&s.id) == s.id
            && is_repo(&s.repo)
            && !s.entries.is_empty()
            && s.entries.iter().all(|e| !e.include.is_empty())
    });
    valid.then(|| {
        file.sources
            .into_iter()
            .map(|s| Source {
                verified: true,
                ..s
            })
            .collect()
    })
}

pub fn bundled() -> Vec<Source> {
    parse_registry(BUNDLED).unwrap_or_default()
}

fn is_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let ok = |p: Option<&str>| {
        p.is_some_and(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

/// Where the registry came from: `remote`, `cache` or `bundled`.
pub async fn registry(github: Option<&GitHub>, cache: &Cache) -> (Vec<Source>, &'static str) {
    let cached = cache.registry();
    if let Some(github) = github {
        match github
            .registry(cached.as_ref().and_then(|c| c.etag.as_deref()))
            .await
        {
            Ok(Fetched::Fresh { body, etag }) => {
                let text = String::from_utf8_lossy(&body).into_owned();
                if let Some(sources) = parse_registry(&text) {
                    let _ = cache.put_registry(&CachedRegistry { etag, body: text });
                    return (sources, "remote");
                }
                tracing::warn!("the remote library registry is not valid; using the cached copy");
            }
            Ok(Fetched::NotModified) => {}
            Err(err) => tracing::info!("library registry not fetched: {}", err.message()),
        }
    }
    match cached.and_then(|c| parse_registry(&c.body)) {
        Some(sources) => (sources, "cache"),
        None => (bundled(), "bundled"),
    }
}

/// A custom source from a GitHub URL (`https://github.com/owner/name`, optionally
/// `/tree/<branch>/<folder>`) and an optional folder; `Err` says what is wrong.
pub fn custom(source: &CustomSource) -> Result<Source, String> {
    let rest = source
        .url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .strip_prefix("https://github.com/")
        .ok_or("use a https://github.com/owner/name URL")?;
    let parts: Vec<&str> = rest.split('/').collect();
    let repo = parts.get(..2).map(|p| p.join("/")).unwrap_or_default();
    if !is_repo(&repo) {
        return Err("use a https://github.com/owner/name URL".into());
    }
    let (branch, mut folder) = match parts.get(2) {
        Some(&"tree") | Some(&"blob") => (
            parts.get(3).map(|b| (*b).to_owned()),
            parts.get(4..).map(|p| p.join("/")).unwrap_or_default(),
        ),
        None => (None, String::new()),
        Some(_) => return Err("use a https://github.com/owner/name URL".into()),
    };
    let extra = source.path.trim().trim_matches('/');
    if !extra.is_empty() {
        folder = if folder.is_empty() {
            extra.to_owned()
        } else {
            format!("{folder}/{extra}")
        };
    }
    if !folder.is_empty() {
        crate::agents::safe_relative(&folder).map_err(|_| "the folder is not a plain path")?;
    }
    let include = if folder.is_empty() {
        "**".to_owned()
    } else {
        format!("{folder}/**")
    };
    let id = slug(&format!("custom-{repo}-{folder}"));
    Ok(Source {
        id,
        name: if folder.is_empty() {
            repo.clone()
        } else {
            format!("{repo}/{folder}")
        },
        repo,
        branch,
        license: String::new(),
        entries: vec![Entry {
            format: None,
            include: vec![include],
            exclude: Vec::new(),
            targets: None,
        }],
        verified: false,
    })
}
