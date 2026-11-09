"""The front doors: Assets for prices or returns, and Portfolio for a portfolio's transactions."""

import datetime
import decimal

import numpy as np
import pandas as pd
import pytest

import finalytics as fl


@pytest.fixture(scope="module")
def prices() -> pd.DataFrame:
    return fl.datasets.asset_class_prices()


def test_the_asset_class_dataset(prices):
    assert list(prices.columns) == ["US Equity", "Intl Equity", "Bonds", "T-Bills", "Gold", "Real Estate"]
    assert prices.index[0] == pd.Timestamp("1996-10-31") and prices.index[-1] == pd.Timestamp("2026-10-30")
    assert (prices.iloc[0] == 100).all()
    table = fl.datasets.asset_class_table()
    assert table.labels == list(prices.columns) and len(table.dates) == len(prices)


def test_assets_from_a_data_frame(prices):
    assets = fl.Assets(prices, risk_free_rate=0.02)
    assert assets.labels == list(prices.columns)
    assert assets.periods_per_year == 12 and len(assets.dates) == len(prices) - 1
    assert assets.returns.shape == (len(prices) - 1, 6)
    expected = prices.pct_change().dropna().to_numpy()
    assert np.allclose(assets.returns, expected, rtol=1e-12)
    stats = assets.statistics()
    assert [s.asset for s in stats] == assets.labels
    assert stats[3].statistics.volatility < 0.01
    assert assets.correlation().labels == assets.labels


def test_the_same_assets_from_csv_returns_and_a_price_table(prices):
    from_frame = fl.Assets(prices)
    from_csv = fl.Assets(fl.datasets.file("asset-classes.csv"))
    from_table = fl.Assets(fl.datasets.asset_class_table())
    returns = prices.pct_change().dropna()
    from_returns = fl.Assets.from_returns(returns)
    for other in (from_csv, from_table, from_returns):
        assert other.labels == from_frame.labels
        assert np.allclose(other.returns, from_frame.returns, rtol=1e-9)
        assert other.periods_per_year == 12


def test_optimising_with_named_limits(prices):
    assets = fl.Assets(prices, risk_free_asset="T-Bills")
    assert assets.risk_free_asset == "T-Bills"
    assert assets.risk_free_rate == pytest.approx(assets.mean_returns()["T-Bills"])
    result = assets.optimise(
        method="maximum_sharpe",
        bounds=(0.0, 0.4),
        asset_bounds={"Gold": (0.0, 0.1)},
        groups={"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)},
    )
    raw = result.results
    weights = raw.weights
    assert list(weights) == ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]
    assert sum(weights.values()) == pytest.approx(1.0)
    assert weights["Gold"] <= 0.1 + 1e-6
    assert 0.3 - 1e-6 <= weights["US Equity"] + weights["Intl Equity"] <= 0.6 + 1e-6
    assert raw.sharpe_ratio == pytest.approx((raw.expected_return - raw.risk_free_rate) / raw.volatility)
    target = assets.optimise(method="target_volatility", target=0.08)
    assert target.results.volatility <= 0.08 + 1e-6
    frontier = assets.frontier(points=5, bounds=(0.0, 0.5))
    assert len(frontier) == 5
    assert set(frontier[0].weights) == set(weights)
    assert all(b.volatility >= a.volatility - 1e-9 for a, b in zip(frontier, frontier[1:]))


def test_mistakes_name_the_option(prices):
    assets = fl.Assets(prices)
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(method="target_return")
    assert [i.path for i in caught.value.issues] == ["options.target"]
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(asset_bounds={"Crypto": (0, 0.1)})
    assert [i.path for i in caught.value.issues] == ["options.asset_bounds.Crypto"]
    for rate in (0.02, 0.0):
        with pytest.raises(fl.ValidationError) as caught:
            fl.Assets(prices, risk_free_rate=rate, risk_free_asset="T-Bills")
        assert [i.path for i in caught.value.issues] == ["options.risk_free_rate"]
    with pytest.raises(fl.ValidationError) as caught:
        fl.Assets(prices, periods_per_year=0, risk_free_rate=float("nan"))
    # Every problem together: the number JSON cannot hold, and the engine's with the other.
    assert [i.path for i in caught.value.issues] == ["options.risk_free_rate", "options.periods_per_year"]
    assert caught.value.issues[0].message == "must be a finite number, not NaN"
    with pytest.raises(fl.ValidationError) as caught:
        fl.Assets(prices, periods_per_year=0)
    assert [i.path for i in caught.value.issues] == ["options.periods_per_year"]
    ewma = fl.Assets(prices, covariance="ewma")
    assert ewma.covariance().method == fl.types.Ewma(decay=0.94)


def test_the_portfolio_dataset():
    portfolio = fl.datasets.portfolio()
    assert portfolio.as_of == datetime.date(2026, 10, 30)
    assert len(portfolio.data.transactions) == 203
    report = portfolio.report(fl.datasets.report_options())
    assert report.attribution is not None and len(report.scenarios.scenarios) == 4
    assert portfolio.weights() == portfolio.weights(portfolio.as_of)


def test_from_holdings_to_trades():
    portfolio = fl.datasets.portfolio()
    assets = portfolio.assets()
    assert assets.labels == ["BND", "EXSA", "GLD", "VNQ", "VTI", "EUR cash", "USD cash"]
    assert assets.periods_per_year == 252
    # Optimisation chooses among the instruments; the cash is there to value the portfolio.
    assert assets.risky_assets == ["BND", "EXSA", "GLD", "VNQ", "VTI"]
    assert assets.cash_assets == ["USD cash", "EUR cash"] and assets.risk_free_asset == "USD cash"
    result = assets.optimise(method="minimum_variance", bounds=(0.05, 0.5))
    proposal = portfolio.rebalance(result)
    same = [portfolio.rebalance(result.results), portfolio.rebalance(result.results.weights)]
    assert all(other.results == proposal.results for other in same)
    assert len(proposal.results.accounts) == 2
    after = portfolio.with_rebalance(proposal).weights()
    assert after == portfolio.with_rebalance(proposal.results).weights()
    for holding, target in result.results.weights.items():
        assert after[holding] == pytest.approx(target, abs=0.01)
    failed = assets.optimise(method="target_return", target=0.9)
    assert failed.results.weights is None
    for targets in (failed, failed.results):
        with pytest.raises(fl.ValidationError) as caught:
            portfolio.rebalance(targets)
        assert [(i.path, i.message) for i in caught.value.issues] == [
            ("targets", "the optimisation found no weights: no portfolio meets every limit")
        ]


def test_limits_on_cash_let_optimisation_hold_it():
    assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    result = assets.optimise(method="minimum_variance", cash=(0, 0.3))
    assert result.results.weights["T-Bills"] == pytest.approx(0.3, abs=1e-6)
    assert result.report.context.limits == ["0% to 30% in cash"]
    # Without a risk-free asset, the cash earns the risk-free rate.
    plain = fl.Assets(fl.datasets.asset_class_prices(), risk_free_rate=0.02)
    held = plain.optimise(method="minimum_variance", cash=(0.1, 0.2)).results.weights
    assert held["Cash"] == pytest.approx(0.2, abs=1e-6)
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(method="maximum_sharpe", cash=(0, 0.2))
    assert caught.value.issues[0].path == "options.cash"


def test_given_weights_are_valued_as_optimisation_values_its_own():
    portfolio = fl.datasets.portfolio()
    valued = portfolio.assets().evaluate(portfolio.weights())
    current = portfolio.report().overview.current
    assert valued.results.volatility == pytest.approx(current.volatility, abs=1e-12)
    assert valued.results.expected_return == pytest.approx(current.expected_return, abs=1e-12)
    assert valued.report.context.given and "Given weights" in str(valued)
    assert list(valued.results.weights)[:5] == ["BND", "EXSA", "GLD", "VNQ", "VTI"]
    assert sum(valued.results.risk_shares.values()) == pytest.approx(1.0, abs=1e-12)
    assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    framed = assets.evaluate({"US Equity": 0.6, "Bonds": 0.4}, max_weight=0.5, frontier=True, name="Sixty forty")
    assert len(framed.report.frontier) == 30 and framed.report.name == "Sixty forty"
    assert "Evaluation · Given weights" in str(framed.results)
    with pytest.raises(fl.ValidationError) as caught:
        assets.evaluate({"US Equity": 0.6}, max_weight=0.5)
    assert [i.path for i in caught.value.issues] == ["options.max_weight", "options.weights"]


def test_a_portfolio_from_transactions_alone():
    transactions = (
        "date,type,instrument,quantity,price,amount,currency\n"
        "2026-01-02,deposit,,,,20000,USD\n"
        "2026-01-02,buy,VOO,20,500,,\n"
        "2026-01-02,buy,AGG,100,98,,\n"
    )
    prices = "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n"
    portfolio = fl.Portfolio.from_csv(transactions=transactions, prices=prices)
    assert portfolio.base_currency == "USD"
    assert [a.id for a in portfolio.data.accounts] == ["main"]
    assert str(portfolio.valuation().total_base) == "20120.0"
    again = fl.Portfolio.read(portfolio.to_json())
    assert again.valuation() == portfolio.valuation()


def test_dataset_files_show_each_layout():
    assert "portfolio/transactions.csv" in fl.datasets.FILES
    header = fl.datasets.file("portfolio/transactions.csv").splitlines()[0]
    assert header.startswith("id,account,trade_date,type,instrument,quantity,price")
    with pytest.raises(fl.ValidationError):
        fl.datasets.file("nothing.csv")


def test_fees_are_an_expense_and_cash_is_riskless():
    portfolio = fl.datasets.portfolio()
    performance = portfolio.performance()
    contributions = performance.contributions()
    expenses = performance.expenses()
    assert expenses < 0
    assert sum(c.contribution for c in contributions) + expenses == pytest.approx(performance.time_weighted(), abs=1e-12)
    usd = next(c for c in contributions if c.holding == "USD cash")
    assert usd.return_base == pytest.approx(0.0, abs=1e-12)
    risk = portfolio.risk_report().holdings
    usd_row = risk.covariance.labels.index("USD cash")
    n = len(risk.covariance.labels)
    assert all(v == 0.0 for v in risk.covariance.values[usd_row * n : (usd_row + 1) * n])


def test_reports_print_and_save_as_html(tmp_path):
    portfolio = fl.datasets.portfolio()
    report = portfolio.report(fl.datasets.report_options())
    text = str(report)
    assert text.startswith("━") and all(len(line) <= 80 for line in text.splitlines())
    assert report.to_text(name="Morgan family portfolio").count("Morgan family portfolio") == 6
    assert str(report.risk).splitlines()[1].startswith("FINALYTICS  ·  RISK")
    assert str(portfolio.performance_report()).splitlines()[1].startswith("FINALYTICS  ·  PERFORMANCE")
    path = report.save_html(tmp_path / "report.html", name="Morgan family portfolio")
    html = path.read_text(encoding="utf-8")
    assert html.startswith("<!doctype html>") and "<title>Morgan family portfolio</title>" in html
    assert "<script" not in html


def test_a_rebalance_has_a_report_and_raw_results():
    portfolio = fl.datasets.portfolio()
    target = portfolio.assets().optimise(method="minimum_variance", bounds=(0.05, 0.40))
    result = portfolio.rebalance(
        target, whole_units=True, fee_bps=5, minimum_fee=1, tax=(0.37, 0.20), name="Morgan family portfolio"
    )
    context = result.report.context
    assert context.fees.rate == decimal.Decimal("0.0005") and context.fees.minimum == 1
    assert (context.tax.short_term, context.tax.long_term) == (decimal.Decimal("0.37"), decimal.Decimal("0.2"))
    assert isinstance(result, fl.types.Rebalancing)
    assert isinstance(result.report, fl.reports.RebalanceReport)
    assert isinstance(result.results, fl.opt.RebalanceResults)
    text = str(result)
    assert text == result.to_text() and "FINALYTICS  ·  REBALANCE" in text and "Morgan family portfolio" in text
    assert all(len(line) <= 80 for line in text.splitlines())
    printed = str(result.results)
    assert printed.startswith("RebalanceResults · 2 accounts")
    assert all(len(line) <= 80 for line in printed.splitlines())
    assert "<title>Morgan family portfolio</title>" in result.to_html() and "Rebalance report" in result.to_html()
    report = result.report
    assert report.context.method == "Minimum variance" and report.context.limits == ["5% to 40% in each asset"]
    assert report.summary.after.volatility < report.summary.before.volatility
    # The retirement account is not taxable: its sales realise gains but pay no tax.
    brokerage, ira = report.accounts
    assert brokerage.taxable and brokerage.tax_base > 0
    assert not ira.taxable and ira.tax_base is None and ira.realised_gain_base > 0
    assert report.summary.tax_base == brokerage.tax_base
    # Purchases leave room for the fees, so no account goes short of cash.
    assert all(a.warnings == [] for a in report.accounts)
    assert all(w >= 0 for w in result.results.weights_after.values())
    assert [t.name for t in fl.reports.tables(report)] == ["holdings", "trades"]
