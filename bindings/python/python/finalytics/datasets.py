"""Illustrative datasets, for examples, tests and practice.

The data is synthetic: drawn from fixed distributions with a fixed seed, so that it gives the
same figures everywhere. It is not the history of any market, index or fund, and it
redistributes no third party's data.

- :func:`asset_class_prices`: 30 years of month-end levels of six asset-class indices.
- :func:`portfolio`: three years of a two-account portfolio in US dollars and euros, and
  :func:`report_options`, the options of its full report.

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

__all__ = ["FILES", "asset_class_prices", "file", "portfolio", "price_table", "report_options"]

FILES: list[str] = _finalytics.dataset_files()
"""The names of the dataset files, for :func:`file`."""


def file(name: str) -> str:
    """The text of a dataset file, by its name in :data:`FILES`, such as
    ``"portfolio/transactions.csv"``."""
    return call(_finalytics.dataset_file, name)


def price_table() -> _types.PriceTable:
    """The asset-class prices of :func:`asset_class_prices`, as a
    :class:`~finalytics.types.PriceTable`, without pandas."""
    return loads(_types.PriceTable, _finalytics.dataset_asset_class_prices())


def asset_class_prices() -> Any:
    """Month-end levels, from October 1996 to October 2026, of six asset-class indices that start
    at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate, as a pandas DataFrame
    indexed by date. The indices include income. Their bear markets are modelled loosely on
    those of 2000–02, 2007–09, 2020 and 2022. (Needs pandas: ``pip install finalytics[pandas]``;
    :func:`price_table` gives the same prices without it.)"""
    import pandas as pd

    table = price_table()
    frame = pd.DataFrame(
        {label: column for label, column in zip(table.labels, table.columns, strict=True)},
        index=pd.DatetimeIndex(table.dates, name="date"),
        dtype=float,
    )
    return frame


def portfolio() -> Portfolio:
    """A portfolio of two accounts held by one person, from November 2023 to October 2026, in US
    dollars: a taxable brokerage account of US and European equity, bond, gold and real estate
    funds, with monthly contributions, a rebalance, an advisory fee and a withdrawal; and a
    tax-deferred retirement account of US equity and bonds. It has dividends (the European
    fund's in euros, with tax withheld), daily prices, a EUR/USD rate, and the levels of a
    balanced benchmark, ``BENCH``: 55% equity, 35% bonds and 10% real assets."""
    return Portfolio._wrap(_finalytics.NativePortfolio.dataset())


def report_options() -> _types.ReportOptions:
    """The options of the full report on :func:`portfolio`: holdings grouped by asset class and
    region, the benchmark ``BENCH`` with its segments by asset class for attribution, a
    risk-free rate of 3%, and four scenarios."""
    return loads(_types.ReportOptions, _finalytics.dataset_report_options())
