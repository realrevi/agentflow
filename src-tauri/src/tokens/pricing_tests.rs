use super::*;
use crate::settings::PriceOverride;

fn tokens(input: u64, output: u64, cache_read: u64, cache_write: u64) -> TokenUsage {
    TokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn shipped_entries_all_have_a_source_and_date() {
    let book = PriceBook::shipped();
    assert!(book.entries().len() > 40);
    for entry in book.entries() {
        assert!(entry.source.starts_with("https://"), "{}", entry.model);
        assert_eq!(entry.updated.len(), 10, "{}", entry.model);
        for price in [
            entry.input,
            entry.output,
            entry.cache_read,
            entry.cache_write,
        ] {
            assert!(price.is_finite() && price >= 0.0, "{}", entry.model);
        }
    }
}

#[test]
fn cost_includes_cache_tokens() {
    let book = PriceBook::shipped();
    // claude-sonnet-4-6: $3 in, $15 out, $0.30 cache read, $3.75 cache write.
    let price = book.price_for("p1", "claude-sonnet-4-6").expect("price");
    let cost = price.cost(&tokens(1_000_000, 100_000, 2_000_000, 400_000));
    assert!(close(cost, 3.0 + 1.5 + 0.6 + 1.5), "{cost}");
    assert!(close(price.cache_savings(&tokens(0, 0, 2_000_000, 0)), 5.4));
}

#[test]
fn model_ids_are_matched_loosely_but_never_guessed() {
    let book = PriceBook::shipped();
    let opus = book.price_for("p1", "claude-opus-4-8");
    assert!(opus.is_some());
    assert_eq!(book.price_for("p1", "anthropic/claude-opus-4.8"), opus);
    assert_eq!(
        book.price_for("p1", "claude-sonnet-4-5-20250929"),
        book.price_for("p1", "claude-sonnet-4-5")
    );
    assert_eq!(
        book.price_for("p1", "models/gemini-2.5-pro"),
        book.price_for("p1", "gemini-2.5-pro")
    );
    assert!(book.price_for("p1", "deepseek-v4-flash").is_some(), "alias");
    assert_eq!(book.price_for("p1", "my-local-llama"), None);
    assert_eq!(book.price_for("p1", "claude-opus-9"), None);
}

#[test]
fn overrides_and_free_providers() {
    let mut book = PriceBook::shipped();
    book.apply_settings(&UsageSettings {
        free_providers: vec!["free-credits".into()],
        prices: vec![PriceOverride {
            model: "my-local-llama".into(),
            input: 1.0,
            output: 2.0,
            cache_read: 0.0,
            cache_write: 0.0,
        }],
        ..UsageSettings::default()
    });
    let custom = book.price_for("p1", "my-local-llama").expect("custom");
    assert!(close(custom.cost(&tokens(500_000, 500_000, 0, 0)), 1.5));
    let free = book
        .price_for("free-credits", "claude-opus-4-8")
        .expect("free");
    assert_eq!(free.cost(&tokens(9_000_000, 9_000_000, 9, 9)), 0.0);
    assert_eq!(
        book.price_for("free-credits", "unknown-model"),
        Some(Price::FREE)
    );
}

#[test]
fn openrouter_prices_are_per_token_strings() {
    let pricing = serde_json::json!({
        "prompt": "0.0000001", "completion": "0.0000005",
        "input_cache_read": "0.00000001", "input_cache_write": "0.000000125"
    });
    let price = openrouter_price(&pricing).expect("price");
    assert!(close(price.input, 0.1) && close(price.output, 0.5));
    assert!(close(price.cache_read, 0.01) && close(price.cache_write, 0.125));
    assert_eq!(
        openrouter_price(&serde_json::json!({ "prompt": "-1" })),
        None
    );
    let mut book = PriceBook::shipped();
    book.set_dynamic("anthropic/claude-haiku-5.5", price);
    assert_eq!(
        book.price_for("or", "anthropic/claude-haiku-5.5"),
        Some(price)
    );
}
