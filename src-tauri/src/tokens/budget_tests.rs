use super::*;
use crate::settings::ProviderBudget;

fn budgets(hard_stop: bool) -> Budgets {
    Budgets {
        daily: Some(10.0),
        monthly: Some(100.0),
        providers: vec![ProviderBudget {
            provider: "p1".into(),
            daily: Some(2.0),
            monthly: None,
        }],
        hard_stop,
    }
}

#[test]
fn levels_at_80_and_100_percent() {
    assert_eq!(level(7.99, 10.0), 0);
    assert_eq!(level(8.0, 10.0), 80);
    assert_eq!(level(10.0, 10.0), 100);
    assert_eq!(level(50.0, 0.0), 0);
}

#[test]
fn statuses_and_crossings() {
    let spent = |scope: Option<&str>, period: Period| match (scope, period) {
        (None, Period::Daily) => 8.5,
        (None, Period::Monthly) => 20.0,
        (Some("p1"), Period::Daily) => 2.5,
        _ => 0.0,
    };
    let list = statuses(&budgets(false), spent);
    assert_eq!(list.len(), 3);
    assert_eq!((list[0].level, list[1].level, list[2].level), (80, 0, 100));

    let mut notified = Notified::default();
    let first = notified.crossings(&list, 100, "2026-10");
    assert_eq!(first.len(), 2, "80% global daily and 100% p1");
    assert!(
        notified.crossings(&list, 100, "2026-10").is_empty(),
        "only once"
    );
    // Global daily goes from 80 to 100: one more warning.
    let mut later = list.clone();
    later[0].level = 100;
    assert_eq!(notified.crossings(&later, 100, "2026-10").len(), 1);
    // A new day starts over.
    assert_eq!(notified.crossings(&list, 101, "2026-10").len(), 2);
    let mut quiet = Notified::default();
    quiet.baseline(&list, 5, "2026-10");
    assert!(quiet.crossings(&list, 5, "2026-10").is_empty());
}

#[test]
fn hard_stop_blocks_only_covered_api_providers() {
    let spent = |scope: Option<&str>, _: Period| if scope == Some("p1") { 3.0 } else { 1.0 };
    let off = budgets(false);
    let on = budgets(true);
    let list = statuses(&on, spent);
    assert!(blocking(&off, &list, "p1").is_none(), "hard stop off");
    assert_eq!(
        blocking(&on, &list, "p1").map(|s| s.scope.as_str()),
        Some("p1")
    );
    assert!(
        blocking(&on, &list, "p2").is_none(),
        "other providers still send"
    );
    let global = statuses(&on, |_, _| 500.0);
    assert_eq!(
        blocking(&on, &global, "p2").map(|s| s.scope.as_str()),
        Some(ALL)
    );
}
