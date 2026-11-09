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

