"""Conversions to pandas, for those who have it installed (``pip install finalytics[pandas]``).

::

    import finalytics.pandas as flpd

    flpd.returns(performance)                  # a Series indexed by date
    flpd.frame(performance.contributions())   # a DataFrame, one row per holding
"""

from __future__ import annotations

import dataclasses
from collections.abc import Iterable
from typing import Any

import pandas as pd

from finalytics import _types
from finalytics._convert import to_json
from finalytics.performance import Performance

__all__ = ["frame", "growth", "label", "returns", "values"]


def returns(performance: Performance) -> pd.Series:
    """The daily returns, indexed by the date each ends on."""
    return pd.Series(performance.returns, index=pd.DatetimeIndex(performance.dates[1:]), name="return")


def growth(performance: Performance) -> pd.Series:
    """The growth of 1 invested on the first date, indexed by date."""
    return pd.Series(performance.growth, index=pd.DatetimeIndex(performance.dates), name="growth")


def values(performance: Performance) -> pd.Series:
    """The portfolio's value on each date, in the base currency."""
    return pd.Series(performance.values, index=pd.DatetimeIndex(performance.dates), name="value")


def label(key: _types.HoldingKey) -> str:
    """A holding as text: the instrument's identifier, or the currency followed by ``cash``."""
    return key.id if isinstance(key, _types.InstrumentHolding) else f"{key.id} cash"


def _cell(value: Any) -> Any:
    if isinstance(value, (_types.InstrumentHolding, _types.CashHolding)):
        return label(value)
    if dataclasses.is_dataclass(value) or isinstance(value, (list, dict)):
        return to_json(value)
    return value


def frame(records: Iterable[Any]) -> pd.DataFrame:
    """A DataFrame with one row per record (any of the dataclasses in :mod:`finalytics.types`)
    and one column per field. Holdings become text labels; nested records become JSON data."""
    rows = [{f.name: _cell(getattr(r, f.name)) for f in dataclasses.fields(r)} for r in records]
    return pd.DataFrame(rows)
