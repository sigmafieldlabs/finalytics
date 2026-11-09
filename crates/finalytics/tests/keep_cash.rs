//! Keeping some cash as it is, rather than investing it: in rebalancing, and in optimisation,
//! whose weights a rebalancing that keeps the same cash then trades to exactly.

#![allow(clippy::unwrap_used)]

use finalytics::{Error, HoldingKey, KeepCash, Method, OptimiseOptions, datasets};

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
fn a_target_for_kept_cash_at_another_weight_and_a_currency_that_is_not_one_are_refused() {
    let portfolio = datasets::portfolio();
    let Err(Error::Validation(found)) = portfolio
        .rebalance([("VTI", 0.5), ("BND", 0.4), ("USD cash", 0.1)])
        .keep_cash(["USD"])
        .build()
    else {
        panic!("a target for kept cash at another weight must be refused")
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

/// The weights a portfolio holds now, by asset label.
fn held(portfolio: &finalytics::Portfolio) -> Vec<(String, f64)> {
    portfolio
        .weights()
        .unwrap()
        .into_iter()
        .map(|(k, w)| (k.to_string(), w))
        .collect()
}

/// Checks that `rebalancing` reaches `optimisation`'s weights: at most `trades` instruments
/// traded, none of them by dust, and every asset at its weight to `tolerance`.
fn reaches(
    optimisation: &finalytics::Optimisation,
    rebalancing: &finalytics::Rebalancing,
    trades: usize,
    tolerance: f64,
) {
    let value: f64 = rebalancing.results.value_base.try_into().unwrap();
    let mut traded: Vec<String> = rebalancing
        .results
        .accounts
        .iter()
        .flat_map(|a| a.trades.iter().map(|t| t.instrument.to_string()))
        .collect();
    traded.sort_unstable();
    traded.dedup();
    assert!(traded.len() <= trades, "{traded:?}");
    for account in &rebalancing.results.accounts {
        for trade in &account.trades {
            let share = f64::try_from(trade.value_base).unwrap() / value;
            assert!(
                share > 1e-6,
                "dust: {} {} {share}",
                trade.side,
                trade.instrument
            );
        }
    }
    let weights = optimisation.results.weights.as_ref().unwrap();
    let after = &rebalancing.results.weights_after;
    for (asset, w) in weights {
        let key: HoldingKey = asset.parse().unwrap();
        let reached = after.get(&key).copied().unwrap_or(0.0);
        assert!(
            (reached - w).abs() < tolerance,
            "{asset}: {reached} for {w}"
        );
    }
}

#[test]
fn the_workstations_optimisation_is_traded_exactly_with_the_cash_kept() {
    // The euro fund is bought with euros converted to the cent, which leaves up to half a cent
    // of the conversion behind: 7e-9 of the household's value. Without a conversion, the
    // weights are reached to the precision of the arithmetic.
    const CENT: f64 = 1e-8;
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    let current = held(&portfolio);
    let optimise = |keep: bool, euro_fund_held: bool| {
        let mut o = assets
            .optimise(Method::MaximumSharpe)
            .current_weights(current.clone())
            .max_trades(4)
            .max_turnover(0.15);
        if keep {
            o = o.keep_all_cash();
        }
        if euro_fund_held {
            let exsa = current.iter().find(|(a, _)| a == "EXSA").unwrap().1;
            o = o.asset_bounds("EXSA", exsa, exsa);
        }
        o.build().unwrap()
    };
    let optimised = |keep: bool| optimise(keep, false);
    let kept = optimised(true);
    let weights = kept.results.weights.as_ref().unwrap();
    // The cash holds its weight exactly; the rest is optimised within the limits.
    for (asset, w) in &current {
        if asset.ends_with(" cash") {
            assert_eq!(weights.get(asset).copied(), Some(*w), "{asset}");
        }
    }
    assert!((weights.values().sum::<f64>() - 1.0).abs() < 1e-12);
    let trades = kept.report.trades.as_ref().unwrap();
    assert!(
        trades.count <= 4 && trades.turnover <= 0.15 + 1e-9,
        "{trades:?}"
    );
    let rebalancing = portfolio.rebalance(&kept).keep_all_cash().build().unwrap();
    reaches(&kept, &rebalancing, 4, CENT);
    // The rebalancing's turnover is the optimisation's, both counting the cash, to what the
    // conversion leaves behind.
    assert!((rebalancing.results.turnover - trades.turnover).abs() < CENT);
    // The same weights, rebalanced without keeping the cash, reach the same portfolio.
    reaches(&kept, &portfolio.rebalance(&kept).build().unwrap(), 4, CENT);
    // Without a conversion, exactly.
    let exact = optimise(true, true);
    let rebalancing = portfolio.rebalance(&exact).keep_all_cash().build().unwrap();
    assert!(
        rebalancing
            .results
            .accounts
            .iter()
            .all(|a| a.conversions.is_empty())
    );
    reaches(&exact, &rebalancing, 4, 1e-12);
    // Without keeping it, the optimisation invests the cash.
    let invested = optimised(false);
    let weights = invested.results.weights.as_ref().unwrap();
    assert!(weights.keys().all(|a| !a.ends_with(" cash")), "{weights:?}");
    let rebalancing = portfolio.rebalance(&invested).build().unwrap();
    reaches(&invested, &rebalancing, 4, CENT);
    let limited = invested.report.trades.as_ref().unwrap().turnover;
    assert!((rebalancing.results.turnover - limited).abs() < CENT);
    reaches(
        &optimise(false, true),
        &portfolio.rebalance(&optimise(false, true)).build().unwrap(),
        4,
        1e-9,
    );
}

#[test]
fn kept_cash_holds_its_weight_and_the_limits_are_of_the_whole_portfolio() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    let current = held(&portfolio);
    let now: std::collections::BTreeMap<String, f64> = current.iter().cloned().collect();
    for method in [
        Method::MinimumVariance,
        Method::MeanVariance,
        Method::MaximumSharpe,
        Method::MinimumCvar,
    ] {
        let mut o = assets
            .optimise(method)
            .current_weights(current.clone())
            .keep_cash(["EUR"])
            .max_weight(0.45);
        if method == Method::MeanVariance {
            o = o.risk_aversion(3.0);
        }
        let result = o.build().unwrap_or_else(|e| panic!("{method:?}: {e}"));
        let w = result.results.weights.as_ref().unwrap();
        assert_eq!(
            w.get("EUR cash").copied(),
            Some(now["EUR cash"]),
            "{method:?}"
        );
        // The dollar cash is not kept, so it is invested.
        assert!(!w.contains_key("USD cash"), "{method:?}: {w:?}");
        assert!(w.values().all(|x| *x <= 0.45), "{method:?}: {w:?}");
        // The frontier holds it too.
        let points = assets
            .optimise(method)
            .current_weights(current.clone())
            .keep_cash(["EUR"])
            .frontier(4)
            .unwrap();
        assert!(
            points
                .iter()
                .all(|p| p.weights.get("EUR cash").copied() == Some(now["EUR cash"]))
        );
    }
    // Kept with cash limits on the other currency's cash, the risk-free asset.
    let result = assets
        .optimise(Method::MinimumVariance)
        .current_weights(current.clone())
        .keep_cash(["EUR"])
        .cash(0.0, 0.1)
        .build()
        .unwrap();
    let w = result.results.weights.unwrap();
    assert_eq!(w.get("EUR cash").copied(), Some(now["EUR cash"]));
    assert!(w["USD cash"] <= 0.1 + 1e-9);
}

#[test]
fn keeping_cash_is_checked_with_the_other_options() {
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    let paths = |r: Result<finalytics::Optimisation, Error>| -> Vec<(String, String)> {
        let Err(Error::Validation(report)) = r else {
            panic!("accepted");
        };
        report
            .issues()
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect()
    };
    let found = paths(
        assets
            .optimise(Method::MinimumVariance)
            .keep_all_cash()
            .max_weight(2.0)
            .build(),
    );
    let at: Vec<&str> = found.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(at, ["options.max_weight", "options.keep_cash"], "{found:?}");
    let found = paths(
        assets
            .optimise(Method::MinimumVariance)
            .current_weights(held(&portfolio))
            .keep_cash(["dollars"])
            .build(),
    );
    assert_eq!(found[0].0, "options.keep_cash[0]");
    let found = paths(
        assets
            .optimise(Method::MinimumVariance)
            .current_weights(held(&portfolio))
            .keep_all_cash()
            .cash(0.0, 0.2)
            .build(),
    );
    assert_eq!(found[0].0, "options.cash");
    assert!(found[0].1.contains("USD cash"), "{found:?}");
    // Methods that take no limits keep no cash.
    let found = paths(
        assets
            .optimise(Method::EqualWeight)
            .current_weights(held(&portfolio))
            .keep_all_cash()
            .build(),
    );
    assert_eq!(
        found,
        [(
            "options.keep_cash".to_owned(),
            "equal weight is fully invested and long only, and takes no limits on weights"
                .to_owned()
        )]
    );
    // Wherever the trade limits are listed, keeping the cash is.
    for info in finalytics::methods() {
        assert_eq!(
            info.takes("keep_cash"),
            info.takes("max_turnover"),
            "{}",
            info.label
        );
    }
}

#[test]
fn a_walk_forward_keeps_the_cash_held_at_each_refit() {
    let assets = finalytics::Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap();
    let kept = assets
        .backtest(Method::MinimumVariance)
        .keep_all_cash()
        .max_turnover(0.5)
        .walk_forward(finalytics::backtest::Years(5))
        .build()
        .unwrap();
    // Fully invested fits hold no cash to keep, so the run is the run without it.
    let plain = assets
        .backtest(Method::MinimumVariance)
        .max_turnover(0.5)
        .walk_forward(finalytics::backtest::Years(5))
        .build()
        .unwrap();
    assert_eq!(
        kept.results.strategies[0].out_of_sample,
        plain.results.strategies[0].out_of_sample
    );
    assert!(
        kept.report
            .context
            .limits
            .iter()
            .any(|l| l == "The cash held kept at each refit")
    );
    // In sample, the weights are fitted once, from cash, with nothing to keep.
    let Err(Error::Validation(found)) = assets
        .backtest(Method::MinimumVariance)
        .keep_all_cash()
        .build()
    else {
        panic!("accepted in sample");
    };
    assert_eq!(found.issues()[0].path, "options.keep_cash");
}

#[test]
fn keeping_no_cash_is_the_same_input_as_keeping_none() {
    // `false` and an empty list keep nothing, as leaving keep_cash out does, so the options are
    // written and digested as if it were left out: Python gives false when it is not given.
    let portfolio = datasets::portfolio();
    let assets = portfolio.assets().unwrap();
    let options = |keep: Option<KeepCash>| {
        let mut options = OptimiseOptions::default();
        options.method = Method::MinimumVariance;
        options.keep_cash = keep;
        options
    };
    let digest = |keep: Option<KeepCash>| {
        let options = options(keep);
        let written = serde_json::to_string(&options).unwrap();
        let digest = assets
            .optimise_with(&options)
            .unwrap()
            .report
            .context
            .digest;
        (written, digest)
    };
    let none = digest(None);
    assert!(none.0.contains(r#""keep_cash":null"#), "{}", none.0);
    assert_eq!(digest(Some(KeepCash::All(false))), none);
    assert_eq!(digest(Some(KeepCash::Currencies(Vec::new()))), none);
    let all = serde_json::to_string(&options(Some(KeepCash::All(true)))).unwrap();
    assert!(all.contains(r#""keep_cash":true"#), "{all}");
    let backtest =
        |b: finalytics::backtest::BacktestBuilder<'_>| b.build().unwrap().report.context.digest;
    assert_eq!(
        backtest(
            assets
                .backtest(Method::MinimumVariance)
                .keep_cash(Vec::<String>::new())
        ),
        backtest(assets.backtest(Method::MinimumVariance)),
    );
}
