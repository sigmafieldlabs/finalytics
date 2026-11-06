"""Datasets for examples, tests and practice.

- :func:`portfolio`: three years of a two-account household in US dollars and euros, holding
  five real funds at their real prices, and :func:`report_options`, the options of its full
  report.
- :func:`asset_class_prices`: 30 years of month-end levels of six asset-class indices,
  synthetic.

The funds' prices and the EUR/USD rate are real, from Yahoo Finance. The household's transactions
and the asset-class indices are illustrative.

Each file is also available as text, through :func:`file`, as an example of the layout
Finalytics reads::

    import finalytics as fl

    print(fl.datasets.file("portfolio/transactions.csv")[:200])
"""

from __future__ import annotations

from typing import Any

from finalytics import _finalytics, _types
from finalytics._convert import loads
from finalytics._errors import call
from finalytics.portfolio import Portfolio

__all__ = [
    "FILES",
    "asset_class_prices",
    "asset_class_table",
    "file",
    "portfolio",
    "report_options",
]

FILES: list[str] = _finalytics.dataset_files()
"""The names of the dataset files, for :func:`file`."""


def file(name: str) -> str:
    """The text of a dataset file, by its name in :data:`FILES`, such as
    ``"portfolio/transactions.csv"``."""
    return call(_finalytics.dataset_file, name)


def _frame(table: _types.PriceTable) -> Any:
    import pandas as pd

    return pd.DataFrame(
        {label: column for label, column in zip(table.labels, table.columns, strict=True)},
        index=pd.DatetimeIndex(table.dates, name="date"),
        dtype=float,
    )


def asset_class_table() -> _types.PriceTable:
    """The asset-class prices of :func:`asset_class_prices`, as a
    :class:`~finalytics.types.PriceTable`, without pandas."""
    return loads(_types.PriceTable, _finalytics.dataset_asset_class_prices())


def asset_class_prices() -> Any:
    """Month-end levels, from October 1996 to October 2026, of six asset-class indices that start
    at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate, as a pandas DataFrame
    indexed by date. The data is synthetic: drawn with a fixed seed, with bear markets modelled
    loosely on those of 2000–02, 2007–09, 2020 and 2022. The indices include income. (Needs
    pandas: ``pip install finalytics[pandas]``; :func:`asset_class_table` gives the same prices
    without it.)"""
    return _frame(asset_class_table())


def portfolio() -> Portfolio:
    """A portfolio of two accounts held by one person, from November 2023 to October 2026, in US
    dollars: a taxable brokerage account of five funds (``VTI``, ``EXSA`` in euros, ``BND``,
    ``GLD`` and ``VNQ``) with monthly contributions, a rebalance, an advisory fee and a
    withdrawal; and a tax-deferred retirement account of US equity and bonds. The prices, the
    funds' distributions and the EUR/USD rate are real; the transactions are illustrative. The
    prices include ``BENCH``, a balanced benchmark built from the funds' total returns: 55%
    equity, 35% bonds and 10% real assets."""
    return Portfolio._wrap(_finalytics.NativePortfolio.dataset())


def report_options() -> _types.ReportOptions:
    """The options of the full report on :func:`portfolio`: holdings grouped by asset class and
    region, the benchmark ``BENCH`` with its segments by asset class for attribution, the
    T-bills' return over the period as the risk-free rate, and four scenarios."""
    return loads(_types.ReportOptions, _finalytics.dataset_report_options())
