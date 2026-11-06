"""Shared inputs: the scenario in ``tests/scenario`` and the S&P 500 prices in ``tests/sp500``,
at the repository root."""

import pathlib
import sys
import urllib.parse
import urllib.request

import pytest

import finalytics as fl

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCENARIO = ROOT / "tests" / "scenario"
SP500 = ROOT / "tests" / "sp500"
FIXTURES = ROOT / "crates"

# Pyodide has no browser for show() to open.
NO_BROWSER = pytest.mark.skipif(sys.platform == "emscripten", reason="no browser to open")


def page_path(url: str) -> pathlib.Path:
    """The file that a ``file:`` URL opened by ``show()`` names, on any platform."""
    return pathlib.Path(urllib.request.url2pathname(urllib.parse.urlparse(url).path))


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
