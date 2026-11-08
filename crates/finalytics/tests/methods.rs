//! One rule for options a method does not use, in `optimise`, `backtest` and `compare`, and the
//! description of each method's options, which the checks follow.

#![allow(clippy::unwrap_used)]

use finalytics::{Assets, Error, Method, OptimiseOptions, ParameterKind, datasets};

fn assets() -> Assets {
    Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()
        .unwrap()
}

fn issues(error: Error) -> Vec<(String, String)> {
    let Error::Validation(report) = error else {
        panic!("{error}")
    };
    report
        .issues()
        .iter()
        .map(|i| (i.path.clone(), i.message.clone()))
        .collect()
}

#[test]
fn one_method_refuses_an_option_it_does_not_use() {
    let assets = assets();
    let found = issues(
        assets
            .optimise(Method::MinimumVariance)
            .target(0.05)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [(
            "options.target".to_owned(),
            "minimum variance does not use target".to_owned()
        )]
    );
    let found = issues(
        assets
            .backtest(Method::EqualWeight)
            .max_weight(0.4)
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].0, "options.max_weight");
    assert!(
        found[0].1.contains("takes no limits on weights"),
        "{found:?}"
    );
    let found = issues(
        assets
            .backtest([("US Equity", 0.6), ("Bonds", 0.4)])
            .max_weight(0.4)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [(
            "options.max_weight".to_owned(),
            "fixed weights take no limits or parameters".to_owned()
        )]
    );
}

#[test]
fn a_comparison_refuses_an_option_no_strategy_uses_and_names_them() {
    let assets = assets();
    let found = issues(
        assets
            .compare([Method::MinimumVariance, Method::MaximumSharpe])
            .target(0.05)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [(
            "options.target".to_owned(),
            "minimum variance and maximum Sharpe do not use target".to_owned()
        )]
    );
    let found = issues(
        assets
            .compare([Method::EqualWeight, Method::RiskParity])
            .max_weight(0.4)
            .build()
            .unwrap_err(),
    );
    assert_eq!(
        found,
        [(
            "options.max_weight".to_owned(),
            "equal weight and risk parity are fully invested and long only, and take no limits \
             on weights"
                .to_owned()
        )]
    );
    let found = issues(
        assets
            .compare::<Method>([])
            .weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])
            .confidence(0.9)
            .build()
            .unwrap_err(),
    );
    assert_eq!(found[0].1, "fixed weights take no limits or parameters");
}

#[test]
fn a_comparison_applies_an_option_to_the_strategies_that_use_it_and_notes_the_others() {
    let assets = assets();
    let result = assets
        .compare([Method::EqualWeight, Method::MinimumVariance])
        .max_weight(0.4)
        .build()
        .unwrap();
    let equal = &result.results.strategies[0];
    assert_eq!(
        equal.notes,
        ["Fully invested and long only; the limits on weights do not apply."]
    );
    assert!(result.results.strategies[1].notes.is_empty());
    let result = assets
        .compare([Method::MinimumCvar, Method::MaximumSharpe])
        .confidence(0.9)
        .build()
        .unwrap();
    assert_eq!(
        result.results.strategies[1].notes,
        ["The confidence does not apply."]
    );
    let text = result.report.to_text(None);
    let words: Vec<&str> = text.split_whitespace().collect();
    assert!(
        words
            .join(" ")
            .contains("Maximum Sharpe: the confidence does not apply."),
        "{text}"
    );
}

/// Values just outside a number's range, refused, and one inside it, taken: at the far end of
/// the range, so that it is a sensible value for the limits too.
fn outside_and_inside(
    min: Option<f64>,
    max: Option<f64>,
    min_x: bool,
    max_x: bool,
) -> Vec<(f64, bool)> {
    let mut out = Vec::new();
    if let Some(m) = min {
        out.push((if min_x { m } else { m - 1e-6 }, false));
    }
    if let Some(m) = max {
        out.push((if max_x { m } else { m + 1e-6 }, false));
        out.push((if max_x { m - 1e-3 } else { m }, true));
    } else if let Some(m) = min {
        out.push((if min_x { m + 1e-3 } else { m }, true));
    }
    out
}

#[test]
fn the_checks_follow_the_methods_descriptions() {
    let assets = assets();
    let methods = finalytics::methods();
    assert_eq!(methods.len(), Method::ALL.len());
    for info in &methods {
        // A method takes no limits on weights exactly when it lists none.
        assert_eq!(info.takes_limits, !info.limits.is_empty(), "{}", info.label);
        // Without a parameter it needs, it is refused at that parameter.
        let mut options = OptimiseOptions::default();
        options.method = info.method;
        if let Err(e) = assets.optimise_with(&options) {
            let required: Vec<String> = info
                .parameters
                .iter()
                .filter(|p| p.required)
                .map(|p| format!("options.{}", p.name))
                .collect();
            let paths: Vec<String> = issues(e).into_iter().map(|(p, _)| p).collect();
            assert_eq!(paths, required, "{}", info.label);
        } else {
            assert!(
                info.parameters.iter().all(|p| !p.required),
                "{}",
                info.label
            );
        }
        // Each number is refused just outside its range and taken just inside.
        for p in info.parameters.iter().chain(&info.limits) {
            if p.kind != ParameterKind::Number && p.kind != ParameterKind::Count {
                continue;
            }
            for (value, ok) in outside_and_inside(p.min, p.max, p.min_exclusive, p.max_exclusive) {
                let mut json = serde_json::json!({ "method": info.method });
                for q in info
                    .parameters
                    .iter()
                    .filter(|q| q.required && q.name != p.name)
                {
                    json[&q.name] = match q.kind {
                        ParameterKind::Uncertainty => {
                            serde_json::json!({ "type": "ellipsoid", "size": 0.1 })
                        }
                        _ => serde_json::json!(0.05),
                    };
                }
                for need in &p.needs {
                    json[need] = match need.as_str() {
                        "current_weights" => serde_json::json!({ "US Equity": 0.5, "Bonds": 0.5 }),
                        _ => serde_json::json!({ "US Equity": 0.6, "Bonds": 0.4 }),
                    };
                }
                json[&p.name] = if p.kind == ParameterKind::Count {
                    serde_json::json!(value.round() as i64)
                } else {
                    serde_json::json!(value)
                };
                if p.kind == ParameterKind::Count && !ok {
                    let below = value.ceil() as i64 - 1;
                    if below < 0 {
                        // A count below zero is not a count: the type refuses it.
                        continue;
                    }
                    json[&p.name] = serde_json::json!(below);
                }
                let options: OptimiseOptions = serde_json::from_value(json.clone()).unwrap();
                let checked = assets.optimise_with(&options);
                let refused = matches!(&checked, Err(Error::Validation(r)) if r.issues().iter().any(|i| i.path == format!("options.{}", p.name)));
                assert_eq!(
                    !refused, ok,
                    "{} {} = {value}: {checked:?}",
                    info.label, p.name
                );
            }
        }
    }
}
