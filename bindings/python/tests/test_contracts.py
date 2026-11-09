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


def _holding(assets, label):
    """Weights that hold `label` at 0.1 and spread the rest over the risky assets."""
    risky = [r for r in assets.risky_assets if r != label]
    weights = {r: 0.9 / len(risky) for r in risky}
    weights[label] = weights.get(label, 0) + 0.1
    return weights


def _labels(assets):
    """A risky asset, the risk-free asset, cash in another currency, and Cash at the rate."""
    return [assets.risky_assets[0], *assets.cash_assets, *([] if assets.risk_free_asset else ["Cash"])]


def _weights_inputs(assets, weights):
    plan = dict(inflation=0.02, end="2030-12", simulations=100)
    return {
        "optimise current_weights": lambda: assets.optimise(method="minimum_variance", current_weights=weights),
        "optimise current_weights with cash": lambda: assets.optimise(
            method="minimum_variance", cash=(0, 0.3), current_weights=weights
        ),
        "optimise benchmark_weights": lambda: assets.optimise(
            method="minimum_variance", benchmark_weights=weights, max_tracking_error=0.5
        ),
        "frontier current_weights": lambda: assets.frontier(points=5, current_weights=weights, max_turnover=0.5),
        "evaluate weights": lambda: assets.evaluate(weights),
        "backtest weights": lambda: assets.backtest(weights=weights),
        "backtest benchmark": lambda: assets.backtest(method="minimum_variance", benchmark=weights),
        "compare weights": lambda: assets.compare(methods=["minimum_variance"], weights={"Mix": weights}),
        "compare benchmark": lambda: assets.compare(methods=["minimum_variance"], benchmark=weights),
        "plan weights": lambda: assets.plan(weights=weights, **plan),
        "plan alternatives": lambda: assets.plan(
            weights={assets.risky_assets[0]: 1}, alternatives={"Alt": weights}, **plan
        ),
    }


def test_every_input_of_weights_takes_every_label_of_the_assets(portfolio, shapes):
    for shape, assets in shapes.items():
        for label in _labels(assets):
            weights = _holding(assets, label)
            for name, run in _weights_inputs(assets, weights).items():
                try:
                    run()
                except fl.ValidationError as e:
                    pytest.fail(f"{shape} {name} {label}: {e}")
            if shape == "household":
                portfolio.rebalance(weights)


def test_every_input_of_weights_refuses_an_unknown_label_in_one_message(portfolio, shapes):
    for shape, assets in shapes.items():
        weights = _holding(assets, "Nope")
        for name, run in _weights_inputs(assets, weights).items():
            with pytest.raises(fl.ValidationError) as found:
                run()
            assert any(
                i.path.endswith(".Nope") and i.message == "unknown asset Nope" for i in found.value.issues
            ), f"{shape} {name}: {found.value}"
    with pytest.raises(fl.ValidationError) as found:
        portfolio.rebalance(_holding(portfolio.assets(), "Nope"))
    assert any(i.path == "targets.Nope" and i.message == "unknown asset Nope" for i in found.value.issues)


def test_bounds_and_groups_refuse_the_cash_in_favour_of_cash_limits(household):
    for label, says in [("USD cash", "limit it with cash"), ("EUR cash", "cash in another currency")]:
        with pytest.raises(fl.ValidationError) as found:
            household.optimise(
                method="minimum_variance", asset_bounds={label: (0, 0.1)}, groups={"G": ([label, "VTI"], 0, 0.9)}
            )
        paths = [i.path for i in found.value.issues]
        assert f"options.asset_bounds.{label}" in paths and "options.groups.G" in paths
        assert all(says in i.message for i in found.value.issues)


def test_the_cash_a_mix_holds_in_another_currency_is_measured_in_every_verb(portfolio, household):
    mix = portfolio.weights()
    assert "EUR cash" in mix
    valued = household.evaluate(mix)
    backtest = household.backtest(weights=mix)
    assert valued.report.history.weights.statistics.volatility == pytest.approx(
        backtest.report.strategy.tested.statistics.volatility, abs=1e-12
    )
    assert "EUR cash" in backtest.results.assets
    traded = household.optimise(method="minimum_variance", current_weights=mix, max_turnover=0.3)
    eur = next(t for t in traded.report.trades.trades if t.asset == "EUR cash")
    assert eur.cash and eur.proposed == 0 and eur.current == pytest.approx(mix["EUR cash"], abs=1e-15)
    assert traded.report.trades.turnover <= 0.3 + 1e-9


def _mixes(assets, portfolio=None):
    """Fixed weights to echo: two risky assets with the cash, and a portfolio's own weights."""
    risky = assets.risky_assets
    cash = assets.risk_free_asset or "Cash"
    mixes = [{risky[0]: 0.6, risky[1]: 0.3, cash: 0.1}, {risky[0]: 0.5, risky[1]: 0.5}]
    if portfolio is not None:
        mixes.append(portfolio.weights())
    return mixes


def test_fixed_weights_come_back_as_given(portfolio, shapes):
    for shape, assets in shapes.items():
        for mix in _mixes(assets, portfolio if shape == "household" else None):
            backtest = assets.backtest(weights=mix)
            assert backtest.report.strategy.weights[0].weights == mix, shape
            compared = assets.compare(methods=["minimum_variance", ("Mix", mix)])
            assert compared.report.strategies[1].weights[0].weights == mix, shape
            against = assets.backtest(method="minimum_variance", benchmark=mix)
            assert against.report.benchmark.weights[0].weights == mix, shape
            compared = assets.compare(methods=["minimum_variance"], benchmark=mix)
            assert compared.report.benchmark.weights[0].weights == mix, shape


def test_a_backtest_is_titled_with_the_name_given(bills):
    method = bills.backtest(method="minimum_variance", name="Mine")
    assert method.report.name == "Mine" and method.report.strategy.name == "Mine"
    fixed = bills.backtest(weights={"US Equity": 0.5, "Bonds": 0.5}, name="Mine")
    assert fixed.report.name == "Mine" and "Mine" in str(fixed)
    assert bills.backtest(method="minimum_variance").report.name is None

