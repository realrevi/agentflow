//! The community agent library (phase 10): finds agents, skills, rules and commands in
//! curated GitHub repositories (`registry/sources.json`) and the user's own sources,
//! converts them into installer templates and hands those to the phase 8 wizard. Nothing
//! from the library is ever run. Sources: docs/library-sources.md.
//!
//! GitHub budget per refresh and source: repository + branch head (ETags) and, only when
//! the commit changed, one recursive tree call; file contents come from
//! raw.githubusercontent.com by commit and are cached by blob sha. Automatic refreshes
//! happen at most once a day; offline, everything is served from the cache.

mod build;
mod cache;
mod catalog;
mod detail;
mod frontmatter;
mod github;
mod import;
mod model;
mod refresh;
mod safety;
mod scalars;
mod select;
mod sources;
mod tomlish;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::providers::Http;
use crate::settings::CustomSource;
use crate::util::{now_ms, run_blocking};
use cache::Cache;
use github::{GhError, GitHub};
use model::{SourceError, SourceStatus};

pub use detail::ItemDetail;
pub use github::Endpoints;
pub use model::Catalog;

/// An automatic refresh waits this long after the last one.
const AUTO_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;

/// Managed state: the cache folder, GitHub's addresses and one refresh at a time.
#[derive(Clone)]
pub struct Library {
    cache: Cache,
    endpoints: Endpoints,
    busy: Arc<tokio::sync::Mutex<()>>,
}

fn source_error(err: &GhError) -> SourceError {
    SourceError {
        kind: err.kind().into(),
        message: err.message(),
        reset_at: match err {
            GhError::RateLimited { reset_at } => Some(*reset_at),
            _ => None,
        },
    }
}

impl Library {
    pub fn new(dir: PathBuf) -> Self {
        Self::with_endpoints(dir, Endpoints::default())
    }

    pub fn with_endpoints(dir: PathBuf, endpoints: Endpoints) -> Self {
        Self {
            cache: Cache::new(dir),
            endpoints,
            busy: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// The cached catalog (never touches the network); before the first refresh, just the
    /// bundled sources.
    pub fn catalog(&self) -> Catalog {
        self.cache.catalog().unwrap_or_else(|| {
            let sources = sources::bundled();
            let mut catalog = catalog::catalog(&sources, &self.cache, &HashMap::new());
            catalog.registry = "bundled".into();
            catalog
        })
    }

    /// Refreshes every source (`force`), or only when the last refresh is a day old.
    pub async fn refresh(
        &self,
        http: Http,
        token: Option<String>,
        custom: &[CustomSource],
        force: bool,
    ) -> AppResult<Catalog> {
        let _one_at_a_time = self.busy.lock().await;
        let previous = self.cache.catalog();
        let recent = previous
            .as_ref()
            .and_then(|c| c.refreshed_at)
            .is_some_and(|at| now_ms().saturating_sub(at) < AUTO_INTERVAL_MS);
        if let (Some(previous), true, false) = (&previous, recent, force) {
            return Ok(previous.clone());
        }
        let rate = previous.as_ref().and_then(|c| c.rate);
        let github = Arc::new(GitHub::new(http, self.endpoints.clone(), token, rate));
        let (mut sources, registry) = sources::registry(Some(&github), &self.cache).await;
        let mut invalid = Vec::new();
        for entry in custom {
            match sources::custom(entry) {
                Ok(source) if !sources.iter().any(|s| s.id == source.id) => sources.push(source),
                Ok(_) => {}
                Err(message) => invalid.push(invalid_status(entry, message)),
            }
        }
        let mut errors = HashMap::new();
        let mut stopped: Option<GhError> = None;
        let mut reached = false;
        for source in &sources {
            if let Some(err) = &stopped {
                errors.insert(source.id.clone(), source_error(err));
                continue;
            }
            match refresh::source(&github, &self.cache, source).await {
                Ok(()) => reached = true,
                Err(err) => {
                    tracing::info!(
                        "library source {} not refreshed: {}",
                        source.id,
                        err.message()
                    );
                    // A used-up limit or no network: the other sources would fail the same.
                    if matches!(err, GhError::RateLimited { .. } | GhError::Network(_)) {
                        stopped = Some(err.clone());
                    }
                    reached |= !matches!(err, GhError::Network(_) | GhError::RateLimited { .. });
                    errors.insert(source.id.clone(), source_error(&err));
                }
            }
        }
        let (cache, rate) = (self.cache.clone(), github.rate());
        let refreshed_at = if reached {
            Some(now_ms())
        } else {
            previous.and_then(|c| c.refreshed_at)
        };
        // Importing and counting tokens of every file: off the async runtime.
        run_blocking(move || {
            let mut catalog = catalog::catalog(&sources, &cache, &errors);
            catalog.sources.extend(invalid);
            catalog.registry = registry.into();
            catalog.rate = rate;
            catalog.refreshed_at = refreshed_at;
            if let Err(err) = cache.put_catalog(&catalog) {
                tracing::warn!("could not save the library catalog: {err}");
            }
            if reached {
                let ids: Vec<String> = sources.iter().map(|s| s.id.clone()).collect();
                cache.prune(&ids);
            }
            Ok(catalog)
        })
        .await
    }

    /// One item in full; a skill's other files are downloaded on first view when online.
    pub async fn detail(
        &self,
        http: Http,
        token: Option<String>,
        id: &str,
    ) -> AppResult<ItemDetail> {
        let catalog = self.catalog();
        let item = catalog
            .items
            .into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| AppError::NotFound(format!("library item {id}")))?;
        let github = Arc::new(GitHub::new(
            http,
            self.endpoints.clone(),
            token,
            catalog.rate,
        ));
        detail::detail(Some(github), &self.cache, item).await
    }
}

fn invalid_status(entry: &CustomSource, message: String) -> SourceStatus {
    SourceStatus {
        id: import::slug(&format!("custom-{}", entry.url)),
        name: entry.url.clone(),
        repo: String::new(),
        verified: false,
        license: String::new(),
        stars: 0,
        commit: None,
        updated_at: None,
        checked_at: None,
        imported: 0,
        unsupported: 0,
        error: Some(SourceError {
            kind: "invalid".into(),
            message,
            reset_at: None,
        }),
    }
}
