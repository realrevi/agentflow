//! Collects watcher events into batches. A batch is due once events pause for `quiet`
//! (200 ms), or after `max_wait` if they never pause (e.g. during a long build).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use super::model::FsChanges;

pub const QUIET: Duration = Duration::from_millis(200);
pub const MAX_WAIT: Duration = Duration::from_secs(1);

pub struct Debouncer {
    quiet: Duration,
    max_wait: Duration,
    paths: BTreeSet<String>,
    structural: BTreeSet<String>,
    rescan: bool,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Debouncer {
    pub fn new(quiet: Duration, max_wait: Duration) -> Self {
        Self {
            quiet,
            max_wait,
            paths: BTreeSet::new(),
            structural: BTreeSet::new(),
            rescan: false,
            first: None,
            last: None,
        }
    }

    pub fn add(&mut self, path: String, structural: bool, now: Instant) {
        if structural {
            self.structural.insert(path.clone());
        }
        self.paths.insert(path);
        self.touch(now);
    }

    pub fn rescan(&mut self, now: Instant) {
        self.rescan = true;
        self.touch(now);
    }

    /// When the pending batch should be sent; `None` while nothing is pending.
    pub fn deadline(&self) -> Option<Instant> {
        let (first, last) = (self.first?, self.last?);
        Some((last + self.quiet).min(first + self.max_wait))
    }

    pub fn take_if_due(&mut self, now: Instant) -> Option<FsChanges> {
        if self.deadline()? > now {
            return None;
        }
        self.first = None;
        self.last = None;
        Some(FsChanges {
            paths: std::mem::take(&mut self.paths).into_iter().collect(),
            structural: std::mem::take(&mut self.structural).into_iter().collect(),
            rescan: std::mem::take(&mut self.rescan),
        })
    }

    fn touch(&mut self, now: Instant) {
        self.first.get_or_insert(now);
        self.last = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn events_within_the_quiet_period_form_one_batch() {
        let start = Instant::now();
        let mut debouncer = Debouncer::new(QUIET, MAX_WAIT);
        assert_eq!(debouncer.deadline(), None);
        debouncer.add("a.txt".into(), true, start);
        debouncer.add("b.txt".into(), false, start + 50 * MS);
        debouncer.add("a.txt".into(), false, start + 120 * MS);

        assert_eq!(
            debouncer.take_if_due(start + 300 * MS),
            None,
            "still settling"
        );
        assert_eq!(debouncer.deadline(), Some(start + 320 * MS));
        let batch = debouncer.take_if_due(start + 320 * MS).expect("due");
        assert_eq!(batch.paths, ["a.txt", "b.txt"]);
        assert_eq!(batch.structural, ["a.txt"]);
        assert!(!batch.rescan);
        assert_eq!(debouncer.deadline(), None, "empty after flushing");
    }

    #[test]
    fn separate_bursts_form_separate_batches() {
        let start = Instant::now();
        let mut debouncer = Debouncer::new(QUIET, MAX_WAIT);
        debouncer.add("one".into(), true, start);
        let first = debouncer.take_if_due(start + 200 * MS).expect("first");
        debouncer.add("two".into(), true, start + 500 * MS);
        let second = debouncer.take_if_due(start + 700 * MS).expect("second");
        assert_eq!(first.paths, ["one"]);
        assert_eq!(second.paths, ["two"]);
    }

    #[test]
    fn a_continuous_stream_is_flushed_after_max_wait() {
        let start = Instant::now();
        let mut debouncer = Debouncer::new(QUIET, MAX_WAIT);
        for i in 0..20 {
            debouncer.add(format!("f{i}"), false, start + i * 100 * MS);
        }
        assert_eq!(debouncer.deadline(), Some(start + MAX_WAIT));
        let batch = debouncer.take_if_due(start + MAX_WAIT).expect("forced");
        assert_eq!(batch.paths.len(), 20);
    }

    #[test]
    fn rescan_alone_is_a_batch() {
        let start = Instant::now();
        let mut debouncer = Debouncer::new(QUIET, MAX_WAIT);
        debouncer.rescan(start);
        let batch = debouncer.take_if_due(start + QUIET).expect("due");
        assert!(batch.rescan && batch.paths.is_empty());
    }
}
