"""Conversions to pandas, for those who have it installed (``pip install finalytics[pandas]``).

::

    import finalytics as fl
    import finalytics.pandas as flpd

    performance = fl.datasets.portfolio().performance()
    universe = fl.Universe(fl.datasets.file("portfolio/prices.csv"), benchmark="BENCH")
    screen = universe.screen()

    flpd.returns(performance)                  # a Series indexed by date
    flpd.frame(performance.contributions())   # a DataFrame, one row per holding
    flpd.securities(screen.results)           # a DataFrame, one row per security
"""

from __future__ import annotations

import dataclasses
from collections.abc import Iterable
from typing import Any

import pandas as pd

from finalytics import _types
from finalytics._convert import to_json
from finalytics.performance import Performance

__all__ = ["frame", "growth", "returns", "securities", "values"]


def returns(performance: Performance) -> pd.Series:
    """The daily returns, indexed by the date each ends on."""
    return pd.Series(performance.returns, index=pd.DatetimeIndex(performance.dates[1:]), name="return")


def growth(performance: Performance) -> pd.Series:
    """The growth of 1 invested on the first date, indexed by date."""
    return pd.Series(performance.growth, index=pd.DatetimeIndex(performance.dates), name="growth")


def values(performance: Performance) -> pd.Series:
    """The portfolio's value on each date, in the base currency."""
    return pd.Series(performance.values, index=pd.DatetimeIndex(performance.dates), name="value")


def _cell(value: Any) -> Any:
    if dataclasses.is_dataclass(value) or isinstance(value, (list, dict)):
        return to_json(value)
    return value


def frame(records: Iterable[Any]) -> pd.DataFrame:
    """A DataFrame with one row per record (any of the dataclasses in :mod:`finalytics.types`)
    and one column per field; nested records become JSON data."""
    rows = [{f.name: _cell(getattr(r, f.name)) for f in dataclasses.fields(r)} for r in records]
    return pd.DataFrame(rows)


def securities(results: _types.ScreenResults | _types.Screen) -> pd.DataFrame:
    """A screen's results as a DataFrame indexed by security, in the universe's order: its
    status, whether it passed every rule and matched, its rank among the matches, its first and
    last dates, one column per measure (NaN where it has no value) and one per field."""
    if isinstance(results, _types.Screen):
        results = results.results
    rows = results.securities
    data: dict[str, Any] = {
        "status": [r.status for r in rows],
        "passed": [r.passed for r in rows],
        "matched": [r.matched for r in rows],
        "rank": pd.array([r.rank for r in rows], dtype="Int64"),
        "first_date": pd.to_datetime([r.first_date for r in rows]),
        "last_date": pd.to_datetime([r.last_date for r in rows]),
    }
    for k, measure in enumerate(results.measures):
        data[measure] = [float("nan") if r.values[k] is None else r.values[k] for r in rows]
    for field in results.fields:
        data[field] = [r.fields.get(field) for r in rows]
    return pd.DataFrame(data, index=pd.Index([r.id for r in rows], name="id"))
