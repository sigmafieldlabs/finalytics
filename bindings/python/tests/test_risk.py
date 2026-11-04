import json

import numpy as np
import pytest

import finalytics as fl

from conftest import FIXTURES

STATISTICS = json.loads((FIXTURES / "finalytics-risk/tests/fixtures/statistics.json").read_text())
COVARIANCE = json.loads((FIXTURES / "finalytics-risk/tests/fixtures/covariance.json").read_text())
DOWNSIDE = json.loads((FIXTURES / "finalytics-risk/tests/fixtures/downside.json").read_text())


@pytest.mark.parametrize("case", STATISTICS["cases"], ids=lambda c: c["name"])
def test_statistics_match_the_reference(case):
    options = case["options"]
    stats = fl.risk.statistics(np.array(case["returns"]), **options)
    for field, expected in case["statistics"].items():
        assert getattr(stats, field) == pytest.approx(expected, rel=1e-10, abs=1e-12), field
    relative = fl.risk.relative_statistics(case["returns"], case["benchmark"], **options)
    for field, expected in case["relative"].items():
        assert getattr(relative, field) == pytest.approx(expected, rel=1e-10, abs=1e-12), field
    for tail in case["tail"]:
        for method in ("historical", "parametric", "cornish_fisher"):
            found = fl.risk.tail_risk(case["returns"], tail["confidence"], method)
            assert [found.value_at_risk, found.expected_shortfall] == pytest.approx(tail[method], rel=1e-9)


def test_invalid_returns_are_reported_by_position():
    with pytest.raises(fl.ValidationError) as caught:
        fl.risk.statistics([0.01, float("nan"), -2.0])
    assert [i.path for i in caught.value.issues] == ["returns[1]", "returns[2]"]


@pytest.mark.parametrize("case", COVARIANCE["cases"], ids=lambda c: c["name"])
def test_covariance_matches_the_reference(case):
    returns = np.array(case["columns"]).T
    for method, key in [("sample", "sample"), ("ewma", "ewma"), ("ledoit_wolf", "ledoit_wolf"), ("oas", "oas")]:
        estimate = fl.risk.covariance(returns, method=method, decay=case["ewma"]["decay"])
        expected = np.array(case[key] if key == "sample" else case[key]["matrix"])
        found = fl.risk.to_array(estimate.matrix).ravel()
        np.testing.assert_allclose(found, expected, rtol=0, atol=1e-12 * np.abs(expected).max())


def test_drawdowns_and_decomposition(books):
    perf = books.performance()
    found = fl.risk.drawdowns(perf.growth, perf.dates)
    worst = min(found, key=lambda d: d.depth)
    assert worst.depth == pytest.approx(fl.risk.drawdown_series(perf.growth, perf.dates).min(), abs=1e-15)

    weights = books.weights("2026-10-30")
    keys = list(weights)
    returns = books.holding_returns(keys)
    columns = np.array(returns.table.columns).T
    estimate = fl.risk.covariance(columns, returns.table.labels, method="ledoit_wolf")
    annual = fl.risk.scaled(estimate.matrix, 252)
    risk = fl.risk.risk_decomposition([weights[k] for k in keys], annual)
    assert sum(c.component for c in risk.contributions) == pytest.approx(risk.volatility, rel=1e-12)
    groups = fl.risk.group_risk(risk, lambda k, label: "Cash" if label.endswith(" cash") else "Invested")
    assert sum(g.share for g in groups) == pytest.approx(1.0, abs=1e-12)
    assert fl.risk.correlation(annual).values[0] == pytest.approx(1.0)
    assert fl.risk.normal_quantile(0.975) == pytest.approx(1.959963984540054, abs=1e-15)


def test_rolling_windows():
    r = np.array(STATISTICS["cases"][1]["returns"])
    rolled = fl.risk.rolling_statistics(r, 12, periods_per_year=12)
    assert len(rolled) == len(r) - 11
    assert rolled[-1] == fl.risk.statistics(r[-12:], periods_per_year=12)
    assert len(fl.risk.rolling_tail_risk(r, 24, 0.9)) == len(r) - 23


@pytest.mark.parametrize("pair", list(zip(STATISTICS["cases"], DOWNSIDE["cases"])), ids=lambda p: p[0]["name"])
def test_downside_measures_match_the_reference(pair):
    case, expected = pair
    options = case["options"]
    stats = fl.risk.statistics(case["returns"], **options)
    assert stats.conditional_drawdown_at_risk == pytest.approx(expected["conditional_drawdown_at_risk"], rel=1e-10)
    assert stats.ulcer_index == pytest.approx(expected["ulcer_index"], rel=1e-12)
    relative = fl.risk.relative_statistics(case["returns"], case["benchmark"], **options)
    for field, value in expected["relative"].items():
        assert getattr(relative, field) == pytest.approx(value, rel=1e-10), field


def test_shortfall_contributions_add_up():
    case = DOWNSIDE["shortfall"]
    scenarios = np.array(case["scenarios"]).T
    found = fl.risk.shortfall_decomposition(case["weights"], scenarios, case["labels"])
    assert found.expected_shortfall == pytest.approx(case["expected_shortfall"], rel=1e-12)
    assert [c.component for c in found.contributions] == pytest.approx(case["contributions"], rel=1e-8)
    assert [c.label for c in found.contributions] == case["labels"]
