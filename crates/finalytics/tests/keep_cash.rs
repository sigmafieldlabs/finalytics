//! Rebalancing that keeps some cash as it is, rather than investing it.

#![allow(clippy::unwrap_used)]

use finalytics::{Error, HoldingKey, Method, datasets};

fn cash(currency: &str) -> HoldingKey {
    format!("{currency} cash").parse().unwrap()
}

#[test]
fn the_cash_kept_stays_and_the_targets_share_the_rest() {
    let portfolio = datasets::portfolio();
    let target = portfolio
        .assets()
        .unwrap()
        .optimise(Method::MinimumVariance)
        .bounds(0.05, 0.4)
        .build()
        .unwrap();
    let invested = portfolio.rebalance(&target).build().unwrap().results;
    let usd = cash("USD");
    let before = invested.weights_before[&usd];
    assert!(before > 0.01, "{before}");
    assert!(invested.weights_after.get(&usd).copied().unwrap_or(0.0) < 1e-6);

    let kept = portfolio
        .rebalance(&target)
        .keep_cash(["USD"])
        .build()
        .unwrap();
    // To within a cent: the euro cash, not kept, is converted to dollars, rounded to cents.
    let after = kept.results.weights_after[&usd];
    let value: f64 = kept.results.value_base.try_into().unwrap();
    assert!((after - before).abs() * value < 0.01, "{before} {after}");
    assert_eq!(kept.results.kept_cash.len(), 1);
    assert!((kept.results.kept_cash[0].weight - before).abs() < 1e-12);
    // The instruments share the rest in the targets' proportions.
    let rest = 1.0
        - before
        - kept
            .results
            .weights_after
            .get(&cash("EUR"))
            .copied()
            .unwrap_or(0.0);
    for (asset, w) in target.results.weights.as_ref().unwrap() {
        let key: HoldingKey = asset.parse().unwrap();
        let got = kept.results.weights_after[&key];
        assert!((got - w * rest).abs() < 1e-6, "{asset}: {got} {}", w * rest);
    }
    let text = kept.report.to_text(None);
    let words: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(words.contains("kept as it is, as asked"), "{text}");
    assert!(text.lines().all(|l| l.chars().count() <= 80));

    // Every currency's cash.
    let all = portfolio
        .rebalance(&target)
        .keep_all_cash()
        .build()
        .unwrap()
        .results;
    let eur = cash("EUR");
    assert!(
        (all.weights_after[&eur] - all.weights_before[&eur]).abs() < 1e-6,
        "{} {}",
        all.weights_before[&eur],
        all.weights_after[&eur]
    );
    assert_eq!(all.kept_cash.len(), 2);
}

#[test]
fn a_target_for_kept_cash_and_a_currency_that_is_not_one_are_refused() {
    let portfolio = datasets::portfolio();
    let Err(Error::Validation(found)) = portfolio
        .rebalance([("VTI", 0.5), ("BND", 0.4), ("USD cash", 0.1)])
        .keep_cash(["USD"])
        .build()
    else {
        panic!("a target for kept cash must be refused")
    };
    assert_eq!(found.issues()[0].path, "targets.USD cash");
    let Err(Error::Validation(found)) = portfolio
        .rebalance([("VTI", 0.5), ("BND", 0.5)])
        .keep_cash(["dollars"])
        .build()
    else {
        panic!("a currency code is needed")
    };
    assert_eq!(found.issues()[0].path, "keep_cash[0]");
}
