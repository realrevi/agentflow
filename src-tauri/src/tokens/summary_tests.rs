use super::*;
use crate::tokens::record::sample;
use crate::tokens::time::parse_iso;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn totals_series_and_breakdowns() {
    let now = parse_iso("2026-10-17T12:00:00Z").expect("now");
    let mut agg = Aggregator::default();
    // claude-sonnet-4-6: $3 in / $15 out per 1M.
    let mut today = sample(
        now - 3_600_000,
        Source::Api,
        "claude-sonnet-4-6",
        1_000_000,
        0,
    );
    today.panel = Some("chat-1".into());
    agg.add(&today, now);
    let mut cli = sample(
        now - 2 * DAY_MS,
        Source::ClaudeCode,
        "claude-sonnet-4-6",
        0,
        1_000_000,
    );
    cli.workspace = Some("/work/other".into());
    agg.add(&cli, now);
    agg.add(
        &sample(now - 10 * DAY_MS, Source::Api, "unknown-llm", 500, 500),
        now,
    );
    let mut compacted = sample(now - 1000, Source::Api, "claude-sonnet-4-6", 0, 100_000);
    compacted.kind = "compact".into();
    agg.add(&compacted, now);

    let book = PriceBook::shipped();
    let summary = summarize(&agg, &book, now, 0);
    assert!(close(summary.today.cost, 3.0 + 1.5));
    assert!(close(summary.week.cost, 3.0 + 1.5 + 15.0));
    assert!(close(summary.week.cli_cost, 15.0));
    assert_eq!(
        summary.month.unpriced_tokens, 1000,
        "unknown model: tokens, no cost"
    );
    assert_eq!(summary.month.requests, 4);
    assert_eq!(summary.days.len(), 30);
    assert_eq!(summary.days[29].date, "2026-10-17");
    assert!(close(summary.days[27].totals.cost, 15.0));
    let sources: Vec<Source> = summary.days[27].sources.iter().map(|s| s.source).collect();
    assert_eq!(
        sources,
        [Source::ClaudeCode],
        "days split by source for the chart"
    );
    assert_eq!(summary.days[29].sources.len(), 1, "today: API only");
    assert!(summary.days[0].sources.is_empty());
    assert_eq!(summary.models[0].key, "claude-sonnet-4-6");
    assert_eq!(
        summary.models[0].source,
        Source::ClaudeCode,
        "most expensive first"
    );
    assert_eq!(summary.panels.len(), 1);
    assert_eq!(summary.panels[0].key, "chat-1");
    assert!(summary.workspaces.iter().any(|w| w.key == "/work/other"));
    assert!(close(summary.compact.cost, 1.5));

    let panel = panel_totals(&agg, &book, "chat-1", now - DAY_MS);
    assert_eq!(panel.tokens.input_tokens, 1_000_000);
    assert!(close(spent(&agg, &book, now - DAY_MS, None), 4.5));
    assert!(close(
        spent(&agg, &book, now - 7 * DAY_MS, Some("claude-code")),
        15.0
    ));
}

#[test]
fn local_offset_moves_the_day_boundary() {
    let now = parse_iso("2026-10-17T00:30:00Z").expect("now");
    let mut agg = Aggregator::default();
    agg.add(
        &sample(now - 3_600_000, Source::Api, "gpt-5", 1_000_000, 0),
        now,
    );
    let book = PriceBook::shipped();
    assert!(
        close(summarize(&agg, &book, now, 0).today.cost, 0.0),
        "UTC: yesterday"
    );
    assert!(
        close(summarize(&agg, &book, now, 120).today.cost, 1.25),
        "UTC+2: today"
    );
}
