"""Optimising, comparing methods and backtesting one allocation: each result has a report and\nthe raw results, prints as text and shows as a page."""

import dataclasses
import datetime
import pathlib
import sys
import types

import pytest
from conftest import NO_BROWSER, page_path

import finalytics as fl


@pytest.fixture(scope="module")
def assets() -> fl.Assets:
    return fl.Assets(prices=fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")


def test_every_result_has_a_report_and_raw_results(assets):
    results = [
        (assets.optimise(method="maximum_sharpe", max_weight=0.40), fl.types.Optimisation,
         fl.reports.OptimisationReport, fl.types.OptimisationResult, "OPTIMISATION"),
        (assets.compare(methods=["equal_weight", "minimum_variance"]), fl.types.Comparison,
         fl.reports.ComparisonReport, fl.types.BacktestResults, "COMPARISON"),
        (assets.backtest(method="minimum_variance"), fl.types.Backtest,
         fl.reports.BacktestReport, fl.types.BacktestResults, "BACKTEST"),
    ]
    for result, kind, report, raw, title in results:
        assert isinstance(result, kind)
        assert isinstance(result.report, report) and isinstance(result.results, raw)
        text = str(result)
        assert text == result.to_text() == str(result.report) and title in text
        assert all(len(line) <= 80 for line in text.splitlines())
        assert result.to_html() == result.report.to_html()
        printed = str(result.results)
        assert printed.startswith(type(result.results).__name__)
        assert all(len(line) <= 80 for line in printed.splitlines())


def test_the_optimisation_report_explains_the_weights(assets):
    result = assets.optimise(method="maximum_sharpe", max_weight=0.40, name="Six asset classes")
    report, raw = result.report, result.results
    assert report.name == "Six asset classes" and "<title>Six asset classes</title>" in result.to_html()
    assert report.summary.volatility == pytest.approx(raw.volatility)
    assert [a.asset for a in report.allocation][0] == report.summary.largest_asset
    assert sum(a.weight for a in report.allocation) == pytest.approx(1.0)
    assert sum(a.risk_share for a in report.allocation) == pytest.approx(1.0)
    assert report.context.method == "Maximum Sharpe" and report.context.limits == ["At most 40% in any asset"]
    assert report.history.weights.statistics.observations == len(assets.dates)
    assert [t.name for t in fl.reports.tables(report)] == ["weights", "correlation"]
    assert "Maximum Sharpe" in str(raw) and "US Equity" in str(raw)
    # The frontier only when asked for: it changes the report, not the weights.
    assert report.frontier == [] and "frontier=True in Python" in result.to_html()
    drawn = assets.optimise(method="maximum_sharpe", max_weight=0.40, frontier=True)
    assert len(drawn.report.frontier) > 10 and drawn.results.weights == raw.weights
    assert "The efficient frontier" in drawn.to_html()


def test_methods_compared_walk_forward(assets):
    result = assets.compare(
        methods=["equal_weight", "minimum_variance", "risk_parity"],
        max_weight=0.40,
        rebalance="quarterly",
        validation=fl.WalkForward(years=5),
        trading_cost_bps=10,
        name="Six asset classes",
    )
    assert isinstance(result, fl.types.Comparison)
    report = result.report
    assert isinstance(report, fl.reports.ComparisonReport) and report.name == "Six asset classes"
    assert [s.name for s in report.strategies] == ["Equal weight", "Minimum variance", "Risk parity"]
    assert report.context.settings.validation == fl.WalkForward(years=5)
    assert report.context.window == 60 and report.context.test_start == assets.dates[60]
    minimum = report.strategies[1]
    assert len(minimum.tested.growth.dates) == len(assets.dates) - 60
    assert minimum.in_sample.start == minimum.tested.start
    assert all(w <= 0.4 + 1e-6 for w in minimum.weights[-1].weights.values())
    assert report.strategies[2].notes == ["Fully invested and long only; the limits on weights do not apply."]
    assert str(result).startswith("━") and "COMPARISON" in str(result)
    assert [t.name for t in fl.reports.tables(report)] == ["statistics", "calendar_years", "weights"]
    assert len(result.results.strategies[1].out_of_sample.returns) == len(assets.dates) - 60
    assert "Out of sample" in str(result.results)
    # The same as the engine reads from JSON.
    same = assets.compare(
        methods=["equal_weight", "minimum_variance", "risk_parity"],
        max_weight=0.40,
        rebalance=fl.types.Quarterly(),
        validation=fl.WalkForward(years=5),
        trading_cost_bps=10,
    )
    assert dataclasses.replace(same.report, name="Six asset classes") == report
    assert same.results == result.results


def test_one_method_or_fixed_weights(assets):
    result = assets.backtest(
        method="maximum_sharpe",
        max_weight=0.40,
        benchmark={"US Equity": 0.60, "Bonds": 0.40},
        validation=fl.Holdout(start="2016-11-01"),
    )
    report = result.report
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
    assert fixed.report.strategy.name == "60/40"
    assert fixed.report.context.settings.rebalance == fl.Drift(band=0.05)
    assert fixed.report.context.settings.validation == fl.InSample()
    assert "<title>60/40</title>" in fixed.to_html()
    # Every return and trade behind the report.
    raw = fixed.results
    assert isinstance(raw, fl.types.BacktestResults)
    assert len(raw.strategies[0].in_sample.returns) == len(assets.dates)
    tested = report.strategy.tested.growth.dates
    assert len(result.results.strategies[0].out_of_sample.returns) == len(tested)


def test_an_index_as_the_benchmark():
    sp500 = fl.Assets(prices=fl.datasets.sp500(), risk_free_asset="T-Bills")
    backtest = sp500.backtest(method="equal_weight", rebalance="yearly", benchmark=fl.datasets.sp500_index()).report
    assert backtest.benchmark.name == "S&P 500"
    assert backtest.benchmark.tested.statistics.observations == len(sp500.dates)
    assert len(backtest.strategy.tested.growth.dates) < 300, "daily returns are charted by month"


def test_mistakes_name_the_option(assets):
    with pytest.raises(fl.ValidationError) as found:
        assets.backtest(name="none")
    assert [(i.path, i.message) for i in found.value.issues] == [("method", "give a method, or fixed weights")]
    with pytest.raises(fl.ValidationError) as found:
        assets.backtest(method="equal_weight", weights={"Bonds": 1.0})
    assert [i.path for i in found.value.issues] == ["weights"]
    with pytest.raises(fl.ValidationError) as found:
        assets.backtest(weights={"Bonds": 1.0}, max_weight=0.4)
    assert [i.path for i in found.value.issues] == ["options.max_weight"]
    with pytest.raises(fl.ValidationError, match="'weekly' is not one of 'monthly'") as found:
        assets.compare(methods=["equal_weight"], rebalance="weekly")
    assert [i.path for i in found.value.issues] == ["rebalance"]
    with pytest.raises(fl.ValidationError) as found:
        assets.compare(methods="minimum_variance")
    assert [(i.path, i.message) for i in found.value.issues] == [
        ("methods", "must be a list of methods, such as ['minimum_variance']")
    ]
    with pytest.raises(fl.ValidationError) as found:
        assets.compare(methods=["minimum_variance"], validation=fl.WalkForward(years=50))
    assert [(i.path, i.message) for i in found.value.issues] == [
        ("validation.years", "the window is 600 periods, but the data has 360 periods: it must leave periods to test")
    ]
    # A number JSON cannot hold, where it is.
    with pytest.raises(fl.ValidationError) as found:
        assets.compare(methods=["minimum_variance"], max_weight=float("nan"))
    assert [i.path for i in found.value.issues] == ["options.max_weight"]
    with pytest.raises(fl.ValidationError) as found:
        assets.optimise(method="minimum_variance", max_weight=float("inf"))
    assert [i.path for i in found.value.issues] == ["options.max_weight"]
    with pytest.raises(TypeError):
        assets.compare(methods=["equal_weight"], objective="maximum_sharpe")
    with pytest.raises(fl.ValidationError) as found:
        assets.backtest(method="equal_weight", validation=fl.Holdout(start="2099-01-01"))
    assert [i.path for i in found.value.issues] == ["validation.start"]
    with pytest.raises(fl.ValidationError) as found:
        assets.optimise(method="risk_parity", budgets={"Bonds": 1.0})
    assert [i.path for i in found.value.issues] == ["options.budgets"]


@NO_BROWSER
def test_a_report_shows_in_a_browser_or_a_notebook(monkeypatch):
    report = fl.datasets.portfolio().report(fl.datasets.report_options(), name="Morgan family portfolio")
    assert report.name == "Morgan family portfolio"
    assert "<title>Morgan family portfolio</title>" in report.to_html()
    assert report.to_text().count("Morgan family portfolio") == 6
    opened = []
    monkeypatch.setattr("webbrowser.open", lambda url: opened.append(url) or True)
    assert report.show() is None
    path = page_path(opened[0])
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
    assert len(shown) == 2 and "Comparison report" in shown[1]
    assets.optimise(method="minimum_variance").show()
    assert len(shown) == 3 and "Optimisation report" in shown[2]
