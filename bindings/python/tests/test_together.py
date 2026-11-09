"""Every problem with a call's inputs is reported together, each at the option given, under
``options`` for every verb: no problem hides another, and a value that cannot be read is
reported with the problems of the others."""

from __future__ import annotations

import math

import pytest

import finalytics as fl

portfolio = fl.datasets.portfolio()
prices = fl.datasets.asset_class_prices()
assets = fl.Assets(prices, risk_free_asset="T-Bills")


def problems(call) -> dict[str, str]:
    with pytest.raises(fl.ValidationError) as caught:
        call()
    return {i.path: i.message for i in caught.value.issues}


CASES = [
    ("four bad options of an optimisation",
     lambda: assets.optimise(method="minimum_cvar", max_weight=2, confidence=5, asset_bounds={"Nope": (0, 1)}, min_effective_assets=0),
     ["options.asset_bounds.Nope", "options.confidence", "options.max_weight", "options.min_effective_assets"]),
    ("a number that is not finite and an unknown asset",
     lambda: assets.optimise(method="minimum_variance", max_weight=math.nan, asset_bounds={"Nope": (0, 1)}),
     ["options.asset_bounds.Nope", "options.max_weight"]),
    ("an unknown method and a limit out of range",
     lambda: assets.optimise(method="minimum_varience", max_weight=2),
     ["options.max_weight", "options.method"]),
    ("weights that are not finite or not assets, and a limit without the frontier",
     lambda: assets.evaluate({"Nope": 0.5, "Bonds": math.nan}, max_weight=0.4),
     ["options.max_weight", "options.weights.Bonds", "options.weights.Nope"]),
    ("fixed weights with an unknown asset and limits",
     lambda: assets.backtest(weights={"Nope": 0.6, "Bonds": 0.4}, max_weight=0.5),
     ["options.max_weight", "options.weights.Nope"]),
    ("a backtest's strategy and settings",
     lambda: assets.backtest(method="minimum_variance", max_weight=2, trading_cost_bps=-5),
     ["options.max_weight", "options.trading_cost_bps"]),
    ("an unknown method among good ones",
     lambda: assets.compare(methods=["minimum_variance", "minimum_varience"], max_weight=2),
     ["options.max_weight", "options.methods[1]"]),
    ("a comparison's fixed weights where they were given",
     lambda: assets.compare(methods=["minimum_variance", ("X", {"Nope": 1.0})], weights={"Y": {"Nope2": 1.0}}, max_weight=2),
     ["options.max_weight", "options.methods[1].weights.Nope", "options.weights.Y.Nope2"]),
    ("a rebalancing's targets and choices",
     lambda: portfolio.rebalance({"Nope": 0.5, "VTI": 0.7}, fee_bps=-1, tax=(2, -1), accounts=["nope"]),
     ["options.accounts[0]", "options.fee_bps", "options.tax[0]", "options.tax[1]", "targets", "targets.Nope"]),
    ("a report's options",
     lambda: portfolio.report(group_by=["nope"], confidences=[2], as_of="1999-01-01", benchmark="NOPE", accounts=["nope"]),
     ["options.accounts[0]", "options.as_of", "options.benchmark", "options.confidences[0]", "options.group_by[0]"]),
    ("a plan's settings",
     lambda: assets.plan(weights={"Nope": 1}, inflation=0.02, end="2030-13", simulations=0, chance=2),
     ["options.chance", "options.end", "options.goal", "options.simulations", "options.weights.Nope"]),
    ("the settings of assets",
     lambda: fl.Assets(prices, risk_free_asset="Nope", periods_per_year=-1, covariance="nope"),
     ["options.covariance", "options.periods_per_year", "options.risk_free_asset"]),
]


@pytest.mark.parametrize(("name", "call", "expected"), CASES, ids=[c[0] for c in CASES])
def test_every_problem_together(name, call, expected):
    assert sorted(problems(call)) == expected, name


def test_problems_are_worded_alike_in_every_language():
    found = problems(lambda: assets.optimise(method="minimum_varience", max_weight=math.nan))
    assert found["options.method"].startswith("minimum_varience is not one of minimum_variance, ")
    assert found["options.max_weight"] == "must be a finite number, not NaN"
