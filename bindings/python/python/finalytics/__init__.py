"""Finalytics: portfolio analytics for Python.

Finalytics keeps the books of a multi-currency portfolio from its transactions, measures
performance and risk, and optimises portfolios under real-world constraints. The engine is
written in Rust; this package is a thin, typed layer over it.

There are two front doors: :class:`Portfolio`, for a portfolio's transactions, and
:class:`Assets`, for the prices or returns of things one might hold::

    import finalytics as fl

    portfolio = fl.Portfolio.from_csv(
        transactions="transactions.csv",
        prices="prices.csv",
    )
    report = portfolio.report(benchmark="SPY")

    assets = fl.Assets(prices, risk_free_rate=0.03)
    result = assets.optimise(objective="maximum_sharpe", bounds=(0.0, 0.4))
"""

from importlib.metadata import version as _distribution_version

from finalytics import datasets, opt, performance, reports, risk, types
from finalytics._errors import FinalyticsError, ValidationError
from finalytics._finalytics import version
from finalytics.assets import Assets
from finalytics.performance import Performance, group_contributions, period
from finalytics.portfolio import (
    Document,
    Holdings,
    MarketData,
    Observed,
    Portfolio,
    PortfolioData,
    cash,
    read_document,
    write_document,
)

__version__: str = _distribution_version("finalytics")
"""The version of this package, in Python's version format (for example ``1.0.0``)."""

__all__ = [
    "Assets",
    "Document",
    "FinalyticsError",
    "Holdings",
    "MarketData",
    "Observed",
    "Performance",
    "Portfolio",
    "PortfolioData",
    "ValidationError",
    "__version__",
    "cash",
    "datasets",
    "group_contributions",
    "opt",
    "performance",
    "period",
    "read_document",
    "reports",
    "risk",
    "types",
    "version",
    "write_document",
]
