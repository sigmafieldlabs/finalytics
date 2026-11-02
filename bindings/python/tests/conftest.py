"""Shared inputs: the scenario in ``tests/scenario`` at the repository root."""

import pathlib

import pytest

import finalytics as fl

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCENARIO = ROOT / "tests" / "scenario"
FIXTURES = ROOT / "crates"


@pytest.fixture(scope="session")
def portfolio() -> fl.PortfolioData:
    return fl.PortfolioData.from_csv(
        instruments=SCENARIO / "instruments.csv",
        transactions=SCENARIO / "transactions.csv",
        base_currency="USD",
    )


@pytest.fixture(scope="session")
def market() -> fl.MarketData:
    return fl.MarketData.from_csv(prices=SCENARIO / "prices.csv", fx=SCENARIO / "fx.csv")


@pytest.fixture(scope="session")
def books(portfolio, market) -> fl.Portfolio:
    return fl.Portfolio(portfolio, market)
