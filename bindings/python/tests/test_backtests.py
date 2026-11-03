"""Comparing methods and backtesting one allocation, and showing a report."""

import dataclasses
import datetime
import pathlib
import sys
import types

import pytest

import finalytics as fl


@pytest.fixture(scope="module")
def assets() -> fl.Assets:
    return fl.Assets(prices=fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")


def test_methods_compared_walk_forward(assets):
    report = assets.compare(
        methods=["equal_weight", "minimum_variance", "risk_parity"],
        max_weight=0.40,
        rebalance="quarterly",
        validation=fl.WalkForward(years=5),
        trading_cost_bps=10,
        name="Six asset classes",
    )
    assert isinstance(report, fl.reports.OptimisationReport) and report.name == "Six asset classes"
    assert [s.name for s in report.strategies] == ["Equal weight", "Minimum variance", "Risk parity"]
    assert report.context.settings.validation == fl.WalkForward(years=5)
    assert report.context.window == 60 and report.context.test_start == assets.dates[60]
    minimum = report.strategies[1]
    assert len(minimum.tested.growth.dates) == len(assets.dates) - 60
    assert minimum.in_sample.start == minimum.tested.start
    assert all(w <= 0.4 + 1e-6 for w in minimum.weights[-1].weights.values())
    assert report.strategies[2].notes == ["Fully invested and long only; the limits on weights do not apply."]
    assert str(report).startswith("━") and "OPTIMISATION" in str(report)
    assert [t.name for t in fl.reports.tables(report)] == ["statistics", "calendar_years", "weights"]
    # The same as the engine reads from JSON.
    same = assets.compare(
        methods=["equal_weight", "minimum_variance", "risk_parity"],
        max_weight=0.40,
        rebalance=fl.types.Quarterly(),
        validation=fl.WalkForward(years=5),
        trading_cost_bps=10,
    )
    assert dataclasses.replace(same, name="Six asset classes") == report


def test_one_method_or_fixed_weights(assets):
    report = assets.backtest(
        method="maximum_sharpe",
        max_weight=0.40,
        benchmark={"US Equity": 0.60, "Bonds": 0.40},
        validation=fl.Holdout(start="2016-11-01"),
    )
    assert isinstance(report, fl.reports.BacktestReport)
    assert report.strategy.name == "Maximum Sharpe" and report.benchmark.name == "Benchmark"
    assert report.strategy.tested.start == datetime.date(2016, 11, 30)
    assert report.history.split == datetime.date(2016, 11, 30)
    assert report.context.settings.validation == fl.Holdout(start=datetime.date(2016, 11, 1))
    assert report.strategy.tested.relative.tracking_error > 0
    fixed = assets.backtest(
        weights={"US Equity": 0.6, "Bonds": 0.4},
        name="60/40",
        rebalance=fl.Drift(band=0.05),
    )
    assert fixed.strategy.name == "60/40" and fixed.context.settings.rebalance == fl.Drift(band=0.05)
    assert fixed.context.settings.validation == fl.InSample()
    assert "<title>60/40</title>" in fixed.to_html()
    # The results alone, without a report.
    raw = assets.simulate(
        fl.types.BacktestOptions(strategies=[fl.types.FixedWeights(name="60/40", weights={"US Equity": 0.6, "Bonds": 0.4})])
    )
    assert isinstance(raw, fl.types.Backtest) and len(raw.strategies[0].in_sample.returns) == len(assets.dates)


def test_an_index_as_the_benchmark():
    sp500 = fl.Assets(prices=fl.datasets.sp500(), risk_free_asset="T-Bills")
    backtest = sp500.backtest(method="equal_weight", rebalance="yearly", benchmark=fl.datasets.sp500_index())
    assert backtest.benchmark.name == "S&P 500"
    assert backtest.benchmark.tested.statistics.observations == len(sp500.dates)
    assert len(backtest.strategy.tested.growth.dates) < 300, "daily returns are charted by month"


def test_mistakes_name_the_option(assets):
    with pytest.raises(ValueError, match="a method or weights"):
        assets.backtest(name="none")
    with pytest.raises(ValueError, match="max_weight"):
        assets.backtest(weights={"Bonds": 1.0}, max_weight=0.4)
    with pytest.raises(ValueError, match="rebalance: 'weekly' is not one of 'monthly'"):
        assets.compare(methods=["equal_weight"], rebalance="weekly")
    with pytest.raises(TypeError):
        assets.compare(methods=["equal_weight"], objective="maximum_sharpe")
    with pytest.raises(fl.ValidationError) as found:
        assets.backtest(method="equal_weight", validation=fl.Holdout(start="2099-01-01"))
    assert [i.path for i in found.value.issues] == ["validation.start"]
    with pytest.raises(fl.ValidationError) as found:
        assets.optimise(method="risk_parity", budgets={"Bonds": 1.0})
    assert [i.path for i in found.value.issues] == ["options.budgets"]


def test_a_report_shows_in_a_browser_or_a_notebook(monkeypatch):
    report = fl.datasets.portfolio().report(fl.datasets.report_options(), name="Morgan family portfolio")
    assert report.name == "Morgan family portfolio"
    assert "<title>Morgan family portfolio</title>" in report.to_html()
    assert report.to_text().count("Morgan family portfolio") == 6
    opened = []
    monkeypatch.setattr("webbrowser.open", lambda url: opened.append(url) or True)
    assert report.show() is None
    path = pathlib.Path(opened[0].removeprefix("file://"))
    assert path.read_text(encoding="utf-8").startswith("<!doctype html>")
    path.unlink()
    frame = report._repr_html_()
    assert frame.startswith("<iframe srcdoc=") and "&lt;title&gt;Morgan family portfolio" in frame


def test_a_report_shows_inline_in_a_notebook(monkeypatch):
    shown = []

    class ZMQInteractiveShell:
        pass

    ipython = types.ModuleType("IPython")
    ipython.get_ipython = ZMQInteractiveShell
    display = types.ModuleType("IPython.display")
    display.HTML = lambda html: html
    display.display = shown.append
    monkeypatch.setitem(sys.modules, "IPython", ipython)
    monkeypatch.setitem(sys.modules, "IPython.display", display)
    monkeypatch.setattr("webbrowser.open", lambda url: pytest.fail("opened a browser"))
    report = fl.datasets.portfolio().report()
    report.show()
    assert len(shown) == 1 and shown[0].startswith("<iframe srcdoc=")
    assets = fl.Assets(prices=fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    assets.compare(methods=["equal_weight", "minimum_variance"]).show()
    assert len(shown) == 2 and "Optimisation report" in shown[1]
