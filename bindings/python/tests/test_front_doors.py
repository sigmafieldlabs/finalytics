"""The front doors: Assets for prices or returns, and Portfolio for a portfolio's transactions."""

import datetime

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
    table = fl.datasets.price_table()
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
    from_table = fl.Assets(fl.datasets.price_table())
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
        objective="maximum_sharpe",
        bounds=(0.0, 0.4),
        asset_bounds={"Gold": (0.0, 0.1)},
        groups={"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)},
    )
    weights = result.weights
    assert list(weights) == ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]
    assert sum(weights.values()) == pytest.approx(1.0)
    assert weights["Gold"] <= 0.1 + 1e-6
    assert 0.3 - 1e-6 <= weights["US Equity"] + weights["Intl Equity"] <= 0.6 + 1e-6
    assert result.sharpe_ratio == pytest.approx((result.expected_return - result.risk_free_rate) / result.volatility)
    target = assets.optimise(objective="target_volatility", target=0.08)
    assert target.volatility <= 0.08 + 1e-6
    frontier = assets.frontier(points=5, bounds=(0.0, 0.5))
    assert len(frontier) == 5
    assert all(b.volatility >= a.volatility - 1e-9 for a, b in zip(frontier, frontier[1:]))


def test_mistakes_name_the_option(prices):
    assets = fl.Assets(prices)
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(objective="target_return")
    assert [i.path for i in caught.value.issues] == ["options.target"]
    with pytest.raises(fl.ValidationError) as caught:
        assets.optimise(asset_bounds={"Crypto": (0, 0.1)})
    assert [i.path for i in caught.value.issues] == ["options.asset_bounds.Crypto"]
    with pytest.raises(ValueError):
        fl.Assets(prices, risk_free_rate=0.02, risk_free_asset="T-Bills")


def test_the_portfolio_dataset():
    portfolio = fl.datasets.portfolio()
    assert portfolio.as_of == datetime.date(2026, 10, 30)
    assert len(portfolio.data.transactions) == 148
    report = portfolio.report(fl.datasets.report_options())
    assert report.attribution is not None and len(report.scenarios.scenarios) == 4
    assert portfolio.weights() == portfolio.weights(portfolio.as_of)


def test_from_holdings_to_trades():
    portfolio = fl.datasets.portfolio()
    assets = portfolio.assets()
    assert assets.labels == ["BOND", "GOLD", "INTL", "REIT", "USEQ"] and assets.periods_per_year == 252
    targets = assets.optimise(objective="minimum_variance", bounds=(0.05, 0.5)).weights
    proposal = portfolio.rebalance(targets)
    assert len(proposal.accounts) == 2
    after = portfolio.with_rebalance(proposal).weights()
    for holding, target in targets.items():
        assert after[holding] == pytest.approx(target, abs=0.01)


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
