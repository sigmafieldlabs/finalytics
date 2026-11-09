"""The contracts between features: each verb takes what the others give it, in every shape of data:
the sample household's assets (five funds, euro cash, and dollar cash as the risk-free asset), the
asset classes with T-bills as the risk-free asset, and the asset classes without them at a 2% rate."""

import pytest

import finalytics as fl


@pytest.fixture(scope="module")
def portfolio():
    return fl.datasets.portfolio()


@pytest.fixture(scope="module")
def household(portfolio):
    return portfolio.assets()


@pytest.fixture(scope="module")
def bills():
    return fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")


@pytest.fixture(scope="module")
def rate():
    return fl.Assets(fl.datasets.asset_class_prices().drop(columns="T-Bills"), risk_free_rate=0.02)


@pytest.fixture(scope="module")
def shapes(household, bills, rate):
    return {"household": household, "bills": bills, "rate": rate}


def test_minimum_cvar_with_cash_reports_the_shortfall_of_every_weight(shapes):
    for shape, assets in shapes.items():
        result = assets.optimise(method="minimum_cvar", cash=(0, 0.2))
        cash = assets.risk_free_asset or "Cash"
        assert cash in result.results.weights, shape
        assert result.report.summary.expected_shortfall is not None, shape
        shares = sum(a.risk_share for a in result.report.allocation)
        assert shares == pytest.approx(1, abs=1e-6), shape
        assert any(a.asset == cash for a in result.report.allocation), shape


def _walk_forward(assets):
    return fl.WalkForward(years=1 if assets.periods_per_year > 100 else 5)


def test_the_engines_own_weights_pass_its_own_checks(portfolio, household, bills):
    cases = [
        dict(method="minimum_cdar"),
        dict(method="mean_variance", risk_aversion=3, max_weight=0.35),
        dict(method="minimum_variance", cash=(0, 0)),
    ]
    for case in cases:
        result = household.optimise(**case)
        weights = result.results.weights
        assert all(w >= 0 for w in weights.values()), case
        assert all(w == 0 or w >= 1e-7 for w in weights.values()), case
        assert sum(weights.values()) == pytest.approx(1, abs=1e-12)
        valued = household.evaluate(weights)
        assert valued.results.volatility == pytest.approx(result.results.volatility, abs=1e-12)
        # A target for rebalancing, and current weights for another optimisation.
        portfolio.rebalance(result)
        household.optimise(method="minimum_variance", current_weights=weights)
    capped = bills.optimise(method="maximum_sharpe", max_weight=0.3)
    assert all(w <= 0.3 for w in capped.results.weights.values())


def test_a_walk_forward_refits_from_the_weights_it_holds_without_checking_them_as_given(shapes):
    for shape, assets in shapes.items():
        wf = _walk_forward(assets)
        assets.backtest(method="mean_variance", risk_aversion=3, min_trade=0.01, validation=wf)
        assets.backtest(method="minimum_cdar", max_turnover=0.5, validation=wf)


def test_a_refit_after_fits_that_held_only_cash_enters_from_cash(bills):
    column = bills.labels.index("US Equity")
    one = fl.Assets.from_returns(
        bills.returns[:, [column]], dates=bills.dates, labels=["US Equity"], risk_free_rate=0.02
    )
    target = one.mean_returns()["US Equity"]
    backtest = one.backtest(method="target_return", target=target, max_turnover=0.5, validation=fl.WalkForward(years=5))
    strategy = backtest.results.strategies[0]
    assert strategy.failures
    assert any(t.refit and t.turnover > 0.5 + 1e-9 for t in strategy.out_of_sample.trades)


def _folded(portfolio, assets):
    """The household's own weights, its euro cash counted with the dollar cash."""
    out = {}
    for key, w in portfolio.weights().items():
        to = key if key in assets.risky_assets else assets.risk_free_asset
        out[to] = out.get(to, 0) + w
    return out


def test_rebalancing_to_an_optimisation_trades_what_it_traded_and_nothing_else(portfolio, household):
    optimisation = household.optimise(
        method="minimum_variance",
        current_weights=_folded(portfolio, household),
        max_turnover=0.1,
        max_trades=2,
        min_trade=0.02,
    )
    traded = sorted(t.asset for t in optimisation.report.trades.trades if not t.cash)
    assert len(traded) == 2
    rebalancing = portfolio.rebalance(optimisation)
    value = float(rebalancing.results.value_base)
    trades = [t for a in rebalancing.results.accounts for t in a.trades]
    assert sorted({t.instrument for t in trades}) == traded
    assert all(float(t.value_base) / value > 1e-6 for t in trades)
    after = rebalancing.results.weights_after
    for asset, w in optimisation.results.weights.items():
        assert after.get(asset, 0) == pytest.approx(w, abs=1e-9), asset


def test_rebalancing_to_the_weights_held_trades_nothing(portfolio):
    rebalancing = portfolio.rebalance(portfolio.weights())
    assert all(not a.trades and not a.conversions for a in rebalancing.results.accounts)


def test_a_change_too_small_to_settle_is_no_trade(portfolio):
    nudged = {}
    sign = 1.0
    for key, w in portfolio.weights().items():
        sign = -sign
        nudged[key] = w + sign * 1e-11 if not key.endswith(" cash") else w
    rebalancing = portfolio.rebalance(nudged)
    assert all(not a.trades for a in rebalancing.results.accounts)
    moved = dict(portfolio.weights())
    moved["VTI"] -= 1e-6
    moved["BND"] += 1e-6
    assert sum(len(a.trades) for a in portfolio.rebalance(moved).results.accounts) >= 2
