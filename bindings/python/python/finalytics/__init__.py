"""Finalytics: portfolio analytics for Python.

Finalytics keeps the books of a multi-currency portfolio from its transactions, measures
performance and risk, and optimises portfolios under real-world constraints. The engine is
written in Rust; this package is a thin, typed layer over it.

There are three front doors: :class:`Portfolio`, for a portfolio's transactions,
:class:`Assets`, for the prices or returns of things one might hold, and :class:`Universe`, for
many securities to screen, each on its own dates::

    import finalytics as fl

    portfolio = fl.Portfolio.from_csv(
        transactions="transactions.csv",
        prices="prices.csv",
        fx="fx.csv",
        instruments="instruments.csv",
        accounts="accounts.csv",
    )
    report = portfolio.report(benchmark="BENCH")

    assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
    result = assets.optimise(method="maximum_sharpe", max_weight=0.4)
    backtest = assets.compare(
        methods=["equal_weight", "minimum_variance", "maximum_sharpe"],
        max_weight=0.4,
        validation=fl.WalkForward(years=5),
    )
    plan = assets.plan(
        fl.Target(amount=150_000, by="2030-06"),
        start_value=60_000,
        flows=[fl.Saving(amount=2_350, every="month")],
        weights={"US Equity": 0.6, "Bonds": 0.4},
        inflation=0.025,
    )

    universe = fl.Universe("prices.csv", fields="instruments.csv", benchmark="BENCH")
    screen = universe.screen(
        rules=[("return_1y", ">", 0.0), ("class.asset_class", "not_in", ["Fixed income"])],
        sort=("sharpe_ratio", "descending"),
        limit=20,
    )

The files are the sample household's, which ``fl.datasets.file("portfolio/prices.csv")`` and the
like give as text.
"""

from importlib.metadata import version as _distribution_version

from finalytics import datasets, opt, performance, reports, risk, types
from finalytics._errors import FinalyticsError, ValidationError
from finalytics._finalytics import version
from finalytics._types import (
    Bootstrap,
    Drift,
    Every,
    FixedMix,
    GlidePath,
    Historical,
    Holdout,
    InSample,
    Normal,
    Payments,
    Saving,
    Spending,
    Target,
    WalkForward,
)
from finalytics.assets import Assets, methods
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
from finalytics.screening import Universe

__version__: str = _distribution_version("finalytics")
"""The version of this package, in Python's version format (for example ``1.0.0``)."""

__all__ = [
    "Assets",
    "Bootstrap",
    "Document",
    "Drift",
    "Every",
    "FinalyticsError",
    "FixedMix",
    "GlidePath",
    "Historical",
    "Holdings",
    "Holdout",
    "InSample",
    "MarketData",
    "Normal",
    "Observed",
    "Payments",
    "Performance",
    "Portfolio",
    "PortfolioData",
    "Saving",
    "Spending",
    "Target",
    "Universe",
    "ValidationError",
    "WalkForward",
    "__version__",
    "cash",
    "datasets",
    "group_contributions",
    "methods",
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
