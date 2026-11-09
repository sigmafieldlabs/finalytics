"""The estimators, downside methods and limits of Assets, each checked against a calculation made
here from the returns."""

import numpy as np
import pytest

import finalytics as fl


@pytest.fixture(scope="module")
def assets() -> fl.Assets:
    return fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")


def portfolio_returns(assets: fl.Assets, weights: dict[str, float]) -> np.ndarray:
    return assets.returns @ np.array([weights.get(label, 0.0) for label in assets.labels])


def paths(error: pytest.ExceptionInfo) -> list[str]:
    return [issue.path for issue in error.value.issues]


def test_the_mean_estimator_sets_the_expected_returns(assets):
    shrunk = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills", mean_returns="james_stein")
    before, after = assets.mean_returns(), shrunk.mean_returns()
    risky = [label for label in assets.labels if label != "T-Bills"]
    grand = np.mean([before[label] for label in risky])
    assert np.mean([after[label] for label in risky]) == pytest.approx(grand, abs=1e-12)
    assert all(abs(after[label] - grand) <= abs(before[label] - grand) + 1e-15 for label in risky)
    assert after["T-Bills"] == before["T-Bills"] == shrunk.risk_free_rate

    weighted = fl.Assets(
        fl.datasets.asset_class_prices(), risk_free_asset="T-Bills", mean_returns=fl.risk.EwmaMean(decay=0.97)
    ).mean_returns()
    columns = assets.returns
    weights = 0.97 ** np.arange(len(columns))[::-1]
    for i, label in enumerate(assets.labels):
        if label != "T-Bills":
            expected = (weights @ columns[:, i]) / weights.sum() * 12
            assert weighted[label] == pytest.approx(expected, rel=1e-12)

    result = shrunk.optimise(method="maximum_sharpe")
    assert "James–Stein mean expected returns" in result.report.to_text()
    with pytest.raises(fl.ValidationError) as caught:
        fl.Assets(fl.datasets.asset_class_prices(), mean_returns="shrunk")
    assert paths(caught) == ["options.mean_returns"]


def test_the_new_covariance_estimators_are_chosen_by_name(assets):
    prices = fl.datasets.asset_class_prices()
    for name in ["constant_correlation", "denoised", "nonlinear_shrinkage", fl.risk.Statistical(factors=2)]:
        chosen = fl.Assets(prices, covariance=name)
        estimate = chosen.covariance()
        assert estimate.matrix.labels == chosen.labels
        assert chosen.optimise(method="minimum_variance").results.status == "optimal"
    with pytest.raises(fl.ValidationError) as caught:
        fl.Assets(prices, covariance="statistical")
    assert paths(caught) == ["options.covariance"]
    with pytest.raises(fl.ValidationError) as caught:
        fl.Assets(prices.iloc[:10], covariance="nonlinear_shrinkage")
    assert paths(caught) == ["options.covariance"]


def test_the_downside_methods_minimise_what_they_say(assets):
    semi = assets.optimise(method="minimum_semivariance")
    returns = portfolio_returns(assets, semi.results.weights)
    assert semi.results.solution.objective_value == pytest.approx(np.mean(np.minimum(returns, 0) ** 2), rel=1e-10)

    cdar = assets.optimise(method="minimum_cdar", confidence=0.9)
    cumulative = np.cumsum(portfolio_returns(assets, cdar.results.weights))
    falls = np.sort(np.maximum.accumulate(np.maximum(cumulative, 0.0)) - cumulative)
    tail = min(z + np.maximum(falls - z, 0).sum() / (0.1 * len(falls)) for z in falls)
    assert cdar.results.solution.objective_value == pytest.approx(tail, rel=1e-10)

    for method in ["minimum_cvar", "minimum_semivariance", "minimum_cdar"]:
        floored = assets.optimise(method=method, target=0.08)
        assert floored.results.status == "optimal"
        assert floored.results.expected_return >= 0.08 - 1e-9
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(method="minimum_semivariance", confidence=0.9)
    assert paths(caught) == ["options.confidence"]


def test_the_effective_number_of_assets_is_held_at_its_floor(assets):
    for method in ["minimum_variance", "maximum_sharpe"]:
        result = assets.optimise(method=method, min_effective_assets=4)
        weights = np.array(list(result.results.weights.values()))
        assert result.results.status == "optimal"
        assert 1 / np.sum(weights**2) >= 4 - 1e-6
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(method="minimum_variance", min_effective_assets=3, asset_bounds={"Gold": (0.6, 1.0)})
    assert paths(caught) == ["options.min_effective_assets"]


def test_resampled_weights_are_the_average_of_the_simulated_optima(assets):
    once = assets.optimise(method="maximum_sharpe", max_weight=0.6)
    resampled = assets.optimise(method="maximum_sharpe", max_weight=0.6, resample=50)
    detail = resampled.results.resampled
    assert detail.solved == 50
    assert list(resampled.results.weights.values()) == pytest.approx(
        [dict(zip(detail.labels, detail.weights))[label] for label in resampled.results.weights], abs=0
    )
    assert resampled.results.solution.weights == once.results.solution.weights
    assert assets.optimise(method="maximum_sharpe", max_weight=0.6, resample=50).results.weights == (
        resampled.results.weights
    )
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(method="equal_weight", resample=50)
    assert paths(caught) == ["options.resample"]


def test_walk_forward_refits_every_so_many_periods(assets):
    result = assets.backtest(
        method="minimum_variance", validation=fl.WalkForward(years=5), rebalance=fl.Every(periods=12)
    )
    run = result.results.strategies[0].out_of_sample
    positions = [run.dates.index(trade.date) for trade in run.trades]
    assert positions == [12 * (k + 1) - 1 for k in range(len(positions))] and positions
    assert "rebalanced every 12 periods" in result.report.to_text()
    with pytest.raises(fl.ValidationError) as caught:
        assets.backtest(method="minimum_variance", rebalance=fl.Every(periods=0))
    assert paths(caught) == ["options.rebalance.periods"]
