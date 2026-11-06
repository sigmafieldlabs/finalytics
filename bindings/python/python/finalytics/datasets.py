"""Datasets for examples, tests and practice.

- :func:`sp500`: daily prices, from January 2005 to October 2026, of 55 S&P 500 companies (the
  five largest in each sector) and of 3-month US Treasury bills; :func:`sp500_index`, the
  S&P 500 total return index; and :func:`sp500_companies`, each company's name, sector and
  industry.
- :func:`portfolio`: three years of a two-account household in US dollars and euros, holding
  five real funds at their real prices, and :func:`report_options`, the options of its full
  report.
- :func:`asset_class_prices`: 30 years of month-end levels of six asset-class indices,
  synthetic.
- :func:`us_cpi`: the US consumer price index, monthly since 1947, for plans in today's money.

The market data is real, from Yahoo Finance, and the price index from the Federal Reserve Bank
of St. Louis (FRED). The S&P 500 companies are today's largest members,
so their history looks back with hindsight (survivorship bias). The household's transactions and
the asset-class indices are illustrative.

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
    "sp500",
    "sp500_companies",
    "sp500_index",
    "sp500_index_table",
    "sp500_table",
    "us_cpi",
    "us_cpi_table",
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


def sp500_table() -> _types.PriceTable:
    """The prices of :func:`sp500`, as a :class:`~finalytics.types.PriceTable`, without pandas."""
    return loads(_types.PriceTable, _finalytics.dataset_sp500_prices())


def sp500() -> Any:
    """Daily prices, from 3 January 2005 to 30 October 2026, of 55 S&P 500 companies, as a pandas
    DataFrame indexed by date: the five largest in each of the 11 sectors, by market value in
    November 2026, among members that traded throughout. The prices are adjusted closes, which
    include dividends and splits. A last column, ``T-Bills``, is the return index of 3-month US
    Treasury bills (the 13-week rate, accrued daily), for use as the risk-free asset.
    :func:`sp500_companies` gives each company's sector.

    The companies are chosen with hindsight: they are today's largest members, so their past
    returns are higher than those of the index's members at the time (survivorship bias).
    (Needs pandas: ``pip install finalytics[pandas]``; :func:`sp500_table` gives the same prices
    without it.)"""
    return _frame(sp500_table())


def sp500_index_table() -> _types.PriceTable:
    """The index of :func:`sp500_index`, as a :class:`~finalytics.types.PriceTable`."""
    return loads(_types.PriceTable, _finalytics.dataset_sp500_index())


def sp500_index() -> Any:
    """The S&P 500 total return index, daily from 3 January 2005 to 30 October 2026, as a pandas
    DataFrame with one column, ``S&P 500``: a benchmark for :func:`sp500`."""
    return _frame(sp500_index_table())


def sp500_companies() -> list[_types.Company]:
    """The companies of :func:`sp500`, in the order of its columns, with their names, GICS
    sectors and sub-industries."""
    return loads(list[_types.Company], _finalytics.dataset_sp500_companies())


def asset_class_table() -> _types.PriceTable:
    """The asset-class prices of :func:`asset_class_prices`, as a
    :class:`~finalytics.types.PriceTable`, without pandas, as :func:`sp500_table` gives the
    S&P 500 prices."""
    return loads(_types.PriceTable, _finalytics.dataset_asset_class_prices())


def asset_class_prices() -> Any:
    """Month-end levels, from October 1996 to October 2026, of six asset-class indices that start
    at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate, as a pandas DataFrame
    indexed by date. The data is synthetic: drawn with a fixed seed, with bear markets modelled
    loosely on those of 2000–02, 2007–09, 2020 and 2022. The indices include income. (Needs
    pandas; :func:`asset_class_table` gives the same prices without it.)"""
    return _frame(asset_class_table())


def us_cpi_table() -> _types.PriceTable:
    """The price index of :func:`us_cpi`, as a :class:`~finalytics.types.PriceTable`, without
    pandas."""
    return loads(_types.PriceTable, _finalytics.dataset_us_cpi())


def us_cpi() -> Any:
    """The US consumer price index for all urban consumers (CPI-U, all items, seasonally
    adjusted, 1982–84 = 100), monthly from January 1947 to September 2026, dated at each month's
    end, as a pandas DataFrame with one column, ``US CPI``. October 2025 is missing: it was not
    published. Give it as the inflation of :meth:`~finalytics.Assets.plan` to draw each month's
    inflation with the same month's returns; a missing month is filled in between its
    neighbours. (Needs pandas; :func:`us_cpi_table` gives the same index without it.)"""
    return _frame(us_cpi_table())


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
