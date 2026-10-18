//! The catalog: every source's items and status from the cache.

use std::collections::HashMap;

use super::build::{source_items, Built};
use super::cache::{Cache, Snapshot};
use super::model::{Catalog, Source, SourceError, SourceStatus};

/// A source's status line from its snapshot, counts and the last error.
pub fn status(
    source: &Source,
    snapshot: Option<&Snapshot>,
    built: &Built,
    error: Option<SourceError>,
) -> SourceStatus {
    SourceStatus {
        id: source.id.clone(),
        name: source.name.clone(),
        repo: source.repo.clone(),
        verified: source.verified,
        license: if source.license.is_empty() {
            snapshot.and_then(|s| s.license.clone()).unwrap_or_default()
        } else {
            source.license.clone()
        },
        stars: snapshot.map_or(0, |s| s.stars),
        commit: snapshot.map(|s| s.commit.clone()).filter(|c| !c.is_empty()),
        updated_at: snapshot.and_then(|s| s.committed_at.clone()),
        checked_at: snapshot.map(|s| s.checked_at).filter(|&t| t > 0),
        imported: built.items.len(),
        unsupported: built.unsupported.len(),
        error,
    }
}

/// The whole catalog from the cache.
pub fn catalog(
    sources: &[Source],
    cache: &Cache,
    errors: &HashMap<String, SourceError>,
) -> Catalog {
    let mut catalog = Catalog::default();
    for source in sources {
        let snapshot = cache.snapshot(&source.id).filter(|s| s.repo == source.repo);
        let built = match &snapshot {
            Some(snapshot) => source_items(source, snapshot, cache),
            None => Built {
                items: Vec::new(),
                unsupported: Vec::new(),
            },
        };
        catalog.sources.push(status(
            source,
            snapshot.as_ref(),
            &built,
            errors.get(&source.id).cloned(),
        ));
        catalog.items.extend(built.items);
        catalog.unsupported.extend(built.unsupported);
    }
    catalog
}
