"""Finalytics: portfolio analytics for Python.

Finalytics keeps the books of a multi-currency portfolio from its transactions, measures
performance and risk, and optimises portfolios under real-world constraints. The engine is
written in Rust; this package is a thin, typed layer over it.

::

    import finalytics as fl

    portfolio = fl.Portfolio.from_csv(
        instruments="instruments.csv", transactions="transactions.csv", base_currency="USD"
    )
    market = fl.MarketData.from_csv(prices="prices.csv", fx="fx.csv")
    books = portfolio.books(market)
    performance = books.performance()
    performance.period("YTD")
    fl.risk.statistics(performance.returns)
"""

from importlib.metadata import version as _distribution_version

from finalytics import opt, risk, types
from finalytics._errors import FinalyticsError, ValidationError
from finalytics._finalytics import version
from finalytics.performance import Performance, group_contributions, period
from finalytics.portfolio import (
    Books,
    Document,
    Holdings,
    MarketData,
    Observed,
    Portfolio,
    cash,
    holding,
    read_document,
    write_document,
)

__version__: str = _distribution_version("finalytics")
"""The version of this package, in Python's version format (for example ``1.0.0``)."""

__all__ = [
    "Books",
    "Document",
    "FinalyticsError",
    "Holdings",
    "MarketData",
    "Observed",
    "Performance",
    "Portfolio",
    "ValidationError",
    "__version__",
    "cash",
    "group_contributions",
    "holding",
    "opt",
    "period",
    "read_document",
    "risk",
    "types",
    "version",
    "write_document",
]
