"""The shared scenario gives the results the Rust tests record in tests/scenario/expected.json."""

import decimal
import json

import numpy as np
import pytest

import finalytics as fl
from finalytics.pandas import label

from conftest import SCENARIO

EXPECTED = json.loads((SCENARIO / "expected.json").read_text())


def results(market, books):
    dates = market.price_dates
    last = dates[-1]
    performance = books.performance(dates)
    returns = performance.returns
    tail = fl.risk.tail_risk(returns, 0.95, "cornish_fisher")
    weights = books.weights(last)
    held = books.holding_returns(list(weights), dates)
    estimate = fl.risk.covariance(np.array(held.table.columns).T, held.table.labels, method="ledoit_wolf")
    decomposition = fl.risk.risk_decomposition(list(weights.values()), fl.risk.scaled(estimate.matrix, 252))
    assets = books.holding_returns(["BOND", "SAP", "SPY"], dates)
    covariance = fl.risk.covariance(np.array(assets.table.columns).T, assets.table.labels, method="sample")
    problem = fl.opt.Problem.from_covariance(fl.risk.scaled(covariance.matrix, 252))
    plan = books.rebalance("main", {"SPY": 0.4, "BOND": 0.4, "SAP": 0.15}, date=last, lot_method="highest_cost")
    return {
        "valuation_total_base": str(books.valuation(last).total_base),
        "realised_gains_base": [str(g.gain_base) for g in books.realised_gains()],
        "time_weighted": performance.time_weighted(),
        "money_weighted": performance.money_weighted(),
        "modified_dietz": performance.modified_dietz(),
        "returns": returns.tolist(),
        "contributions": {label(c.holding): c.contribution for c in performance.contributions()},
        "statistics": fl._convert.to_json(fl.risk.statistics(returns)),
        "tail_risk": [tail.value_at_risk, tail.expected_shortfall],
        "weights": {label(k): w for k, w in weights.items()},
        "shrinkage": estimate.shrinkage,
        "volatility": decomposition.volatility,
        "risk_shares": [c.share for c in decomposition.contributions],
        "minimum_variance": problem.solve(fl.opt.MinimumVariance()).weights,
        "hierarchical_risk_parity": problem.solve(fl.opt.HierarchicalRiskParity()).weights,
        "rebalance_trades": [[t.instrument, t.side, str(t.quantity)] for t in plan.trades],
        "rebalance_realised_gain_base": str(plan.realised_gain_base),
    }


def compare(actual, expected, path):
    if isinstance(expected, dict):
        assert sorted(actual) == sorted(expected), path
        for key in expected:
            compare(actual[key], expected[key], f"{path}.{key}")
    elif isinstance(expected, list):
        assert len(actual) == len(expected), path
        for k, (a, e) in enumerate(zip(actual, expected)):
            compare(a, e, f"{path}[{k}]")
    elif isinstance(expected, float):
        assert actual == pytest.approx(expected, rel=1e-12, abs=1e-300), path
    elif isinstance(expected, str) and _is_decimal(expected):
        assert decimal.Decimal(actual) == decimal.Decimal(expected), path
    else:
        assert actual == expected, path


def _is_decimal(text):
    try:
        decimal.Decimal(text)
    except decimal.InvalidOperation:
        return False
    return True


def test_the_scenario_matches_the_rust_results(market, books):
    compare(results(market, books), EXPECTED, "results")
