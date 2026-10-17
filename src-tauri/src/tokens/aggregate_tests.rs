use super::*;
use crate::tokens::record::sample;

const NOW: u64 = 1_800_000_000_000;

#[test]
fn adds_dedups_and_visits() {
    let mut agg = Aggregator::default();
    let mut a = sample(NOW - 1000, Source::ClaudeCode, "m", 10, 5);
    a.id = "msg_1".into();
    assert!(agg.is_new(&a));
    assert!(agg.add(&a, NOW));
    assert!(!agg.is_new(&a));
    assert!(!agg.add(&a, NOW), "same id twice");
    let mut other_source = a.clone();
    other_source.source = Source::Codex;
    assert!(agg.add(&other_source, NOW), "ids are per source");
    let no_id = sample(NOW - 2000, Source::Api, "m", 1, 1);
    assert!(
        agg.add(&no_id, NOW) && agg.add(&no_id, NOW),
        "no id: no dedup"
    );

    let mut total = Counts::default();
    agg.visit(NOW - DAY_MS, NOW + 1, |_, _, counts| {
        total.add(&counts.tokens, counts.requests)
    });
    assert_eq!(total.tokens.input_tokens, 22);
    assert_eq!(total.requests, 4);
}

#[test]
fn old_records_only_count_for_dedup_and_prune_drops_buckets() {
    let mut agg = Aggregator::default();
    let mut old = sample(NOW - 100 * DAY_MS, Source::Codex, "m", 10, 5);
    old.id = "resp_old".into();
    assert!(agg.add(&old, NOW));
    assert!(!agg.add(&old, NOW));
    let recent = sample(NOW - 61 * DAY_MS, Source::Api, "m", 1, 1);
    agg.add(&recent, NOW);
    let mut count = 0;
    agg.visit(0, NOW, |_, _, _| count += 1);
    assert_eq!(count, 1);
    agg.prune(NOW + 2 * DAY_MS);
    let mut count = 0;
    agg.visit(0, NOW + 3 * DAY_MS, |_, _, _| count += 1);
    assert_eq!(count, 0);
}
