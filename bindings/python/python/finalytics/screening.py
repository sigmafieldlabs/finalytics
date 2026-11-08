"""Screening: measures and rules on many securities, each on its own history."""

from __future__ import annotations

import dataclasses
import json
import math
import os
import typing
from collections.abc import Mapping, Sequence
from typing import Any

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._convert import dumps, loads
from finalytics._errors import call, invalid

__all__ = ["Universe"]

RuleLike = _types.Rule | tuple[str, str, Any] | Mapping[str, Any]
"""A rule: a ``(name, op, value)`` triple, such as ``("return_1y", ">", 0.1)``,
``("rsi_14", "between", (30, 70))`` or ``("sector", "in", ["Energy", "Utilities"])``, or a
:class:`~finalytics.types.Rule`. A name that is a measure tests the measure; any other names a
field."""

SortLike = _types.Sort | tuple[str, str]
"""How matches are sorted: a ``(name, order)`` pair, such as ``("sharpe_ratio", "descending")``,
or a :class:`~finalytics.types.Sort`."""

_MEASURES = set(typing.get_args(_types.Measure)) | {"first_date", "last_date"}
_ID = ("id", "instrument", "ticker", "symbol")


def _target(name: str) -> dict[str, str]:
    """`name` as the measure it names, or else as a field."""
    return {"measure": name} if name in _MEASURES else {"field": name}


def _rule(rule: RuleLike, k: int) -> _types.Rule:
    if isinstance(rule, _types.Rule):
        return rule
    if isinstance(rule, Mapping):
        return loads(_types.Rule, json.dumps(dict(rule)))
    try:
        name, op, value = rule
    except (TypeError, ValueError):
        raise invalid((f"rules[{k}]", "give a rule as (name, op, value), such as (\"return_1y\", \">\", 0.1)")) from None
    target = _target(name)
    if op in ("in", "not_in"):
        values = [value] if isinstance(value, str) else [str(v) for v in value]
        return _types.Rule(field=name, op=op, values=values)
    if op == "between":
        low, high = value
        return _types.Rule(**target, op=op, value=[float(low), float(high)])
    return _types.Rule(**target, op=op, value=float(value))


def _sort(sort: SortLike | None) -> _types.Sort | None:
    if sort is None or isinstance(sort, _types.Sort):
        return sort
    if isinstance(sort, str):
        raise invalid(("sort", f"give the order too, such as ({sort!r}, \"descending\")"))
    name, order = sort
    return _types.Sort(**_target(name), order=order)


def _scalar(value: Any) -> Any:
    """A field's value as plain Python: a number, text, or None when missing."""
    if value is None:
        return None
    if hasattr(value, "item"):
        value = value.item()
    if isinstance(value, float) and math.isnan(value):
        return None
    if isinstance(value, (bool, int, float)):
        return float(value)
    return str(value)


def _columns(frame: Any) -> dict[str, str]:
    """The columns of a DataFrame by their lower-case names."""
    return {str(c).strip().lower(): c for c in frame.columns}


def _long_frame(frame: Any) -> tuple:
    """Flat columns from a long DataFrame of ``id``, ``date`` and ``close``, with ``volume`` and
    ``dividends`` if given, in any order: each security's rows sorted by date, the securities in
    the order the frame first names them."""
    import pandas as pd

    columns = _columns(frame)
    if "date" not in columns and frame.index.name is not None and str(frame.index.name).lower() == "date":
        frame = frame.reset_index()
        columns = _columns(frame)
    id_column = next((columns[c] for c in _ID if c in columns), None)
    close_column = columns.get("close", columns.get("price"))
    missing = [name for name, c in [("id", id_column), ("date", columns.get("date")), ("close", close_column)] if c is None]
    if missing:
        raise invalid(*((f"prices.{m}", "is a column a long DataFrame needs") for m in missing))
    codes, ids = pd.factorize(frame[id_column], sort=False)
    dates = frame[columns["date"]]
    if not pd.api.types.is_datetime64_any_dtype(dates):
        dates = pd.to_datetime(dates)
    days = dates.to_numpy().astype("datetime64[D]").astype(np.int64)
    # Rows already grouped by security and in date order, as most tables are, need no sorting.
    same = codes[1:] == codes[:-1]
    sorted_already = bool(np.all(codes[1:] >= codes[:-1]) and np.all(days[1:][same] > days[:-1][same]))
    order = None if sorted_already else np.lexsort((days, codes))
    if order is not None:
        codes, days = codes[order], days[order]
    starts = np.searchsorted(codes, np.arange(len(ids)))

    def values(name: str | None) -> np.ndarray | None:
        if name is None:
            return None
        column = frame[name]
        if not pd.api.types.is_float_dtype(column):
            column = pd.to_numeric(column, errors="coerce")
        values = column.to_numpy(dtype=np.float64, na_value=np.nan)
        return values if sorted_already else values[order]

    dividends = columns.get("dividends", columns.get("dividend"))
    return (
        [str(i) for i in ids],
        starts,
        days,
        values(close_column),
        values(columns.get("volume")),
        values(dividends),
    )


def _mapping(prices: Mapping[str, Any]) -> tuple:
    """Flat columns from a mapping of each security to a pandas Series of closes indexed by date,
    or to a DataFrame of ``close``, with ``volume`` and ``dividends`` if given."""
    ids, starts, days, close, volume, dividends = [], [], [], [], [], []
    any_volume = any_dividends = False
    for name, series in prices.items():
        if hasattr(series, "columns"):
            columns = _columns(series)
            frame = series.sort_index()
            if "close" not in columns:
                raise invalid((f"prices.{name}", "a DataFrame of one security needs a close column"))
            c = frame[columns["close"]]
            v = frame[columns["volume"]] if "volume" in columns else None
            d = frame[columns["dividends"]] if "dividends" in columns else None
        elif hasattr(series, "index"):
            frame = series.sort_index()
            c, v, d = frame, None, None
        else:
            raise invalid((f"prices.{name}", "must be a pandas Series indexed by date, or a DataFrame"))
        n = len(c)
        ids.append(str(name))
        starts.append(sum(len(x) for x in days))
        days.append(np.asarray(c.index.to_numpy().astype("datetime64[D]").astype(np.int64)))
        close.append(np.asarray(c, dtype=np.float64))
        any_volume |= v is not None
        any_dividends |= d is not None
        volume.append(np.full(n, np.nan) if v is None else np.asarray(v, dtype=np.float64))
        dividends.append(None if d is None else np.asarray(d, dtype=np.float64))
    if not ids:
        raise invalid(("prices", "give at least one security"))
    join = lambda parts: np.concatenate(parts) if parts else np.zeros(0)  # noqa: E731
    return (
        ids,
        np.asarray(starts, dtype=np.int64),
        join(days),
        join(close),
        join(volume) if any_volume else None,
        join([np.zeros(len(c)) if d is None else d for c, d in zip(close, dividends)]) if any_dividends else None,
    )


def _fields(fields: Any) -> dict[str, dict[str, Any]]:
    """Fields by security and then by name: from a DataFrame indexed by security, with one column
    per field, or a mapping of each security to its fields."""
    if hasattr(fields, "columns") and hasattr(fields, "index"):
        out: dict[str, dict[str, Any]] = {}
        for column in fields.columns:
            for key, value in zip(fields.index, fields[column]):
                v = _scalar(value)
                if v is not None:
                    out.setdefault(str(key), {})[str(column)] = v
        return out
    if isinstance(fields, Mapping):
        return {
            str(k): {str(n): v for n, v in ((n, _scalar(x)) for n, x in values.items()) if v is not None}
            for k, values in fields.items()
        }
    raise TypeError(
        "fields must be a pandas DataFrame indexed by security, a mapping of each security to its "
        "fields, or a CSV file or text"
    )


class Universe:
    """Many securities, each with its own dates and closes, and their volumes, dividends and
    fields where known, for :meth:`screen`.

    `prices` is one of:

    - a pandas DataFrame in the long layout: ``id``, ``date`` and ``close`` columns, with
      ``volume`` and ``dividends`` if known, one row per security and date, in any order;
    - a mapping of each security to a pandas Series of closes indexed by date, or to a DataFrame
      of ``close``, ``volume`` and ``dividends`` columns;
    - a CSV file, given as a path or as text: the long layout, or a ``date`` column and one
      column of closes per security, empty where a security has none.

    A close that is not a number is no close, and its date is left out with a warning.

    - `fields`: values the library does not compute, such as a sector or a price-to-earnings
      ratio, for rules and results: a DataFrame indexed by security with one column per field,
      a mapping such as ``{"AAPL": {"sector": "Technology", "pe_ratio": 31.2}}``, or a CSV file
      or text with an ``id`` column.
    - `benchmark`: one of the securities, such as an index, for beta, correlation and relative
      returns.
    - `risk_free_rate`: the annual rate of the Sharpe and Sortino ratios; 0 by default.

    ::

        import finalytics as fl

        universe = fl.Universe("prices.csv", fields="instruments.csv", benchmark="BENCH")
        screen = universe.screen(rules=[("return_1y", ">", 0)], sort=("sharpe_ratio", "descending"))
        screen.show()
    """

    def __init__(
        self,
        prices: Any,
        *,
        fields: Any = None,
        benchmark: str | None = None,
        risk_free_rate: float = 0.0,
    ):
        if isinstance(prices, (str, os.PathLike)):
            from finalytics.portfolio import _text

            native = call(_finalytics.NativeUniverse.from_csv, _text(prices))
        else:
            if hasattr(prices, "columns") and hasattr(prices, "index"):
                ids, starts, days, close, volume, dividends = _long_frame(prices)
            elif isinstance(prices, Mapping):
                ids, starts, days, close, volume, dividends = _mapping(prices)
            else:
                raise TypeError(
                    "prices must be a long pandas DataFrame (id, date, close), a mapping of each "
                    "security to its closes, or a CSV file or text"
                )
            native = call(
                _finalytics.NativeUniverse.from_columns,
                json.dumps(ids),
                np.ascontiguousarray(starts, dtype="<i8").tobytes(),
                np.ascontiguousarray(days, dtype="<i4").tobytes(),
                _arrays.floats_to_buffer(close),
                None if volume is None else _arrays.floats_to_buffer(volume),
                None if dividends is None else _arrays.floats_to_buffer(dividends),
            )
        if fields is not None:
            if isinstance(fields, (str, os.PathLike)):
                from finalytics.portfolio import _text

                native = call(native.with_fields_csv, _text(fields))
            else:
                native = call(native.with_fields, dumps(_fields(fields), "fields"))
        settings = {"benchmark": benchmark, "risk_free_rate": float(risk_free_rate)}
        self._native = call(native.with_settings, dumps(settings))

    @property
    def ids(self) -> list[str]:
        """The securities, in order."""
        return self._native.ids()

    @property
    def benchmark(self) -> str | None:
        """The benchmark, if one is set."""
        return self._native.benchmark()

    @property
    def risk_free_rate(self) -> float:
        """The annual risk-free rate."""
        return self._native.risk_free_rate()

    @property
    def fields(self) -> list[str]:
        """The names of the fields given."""
        return self._native.fields()

    @property
    def latest_date(self) -> str | None:
        """The latest date of any security's closes: the date a screen measures to by default."""
        return self._native.latest_date()

    @property
    def warnings(self) -> list[_types.Issue]:
        """Problems met reading the securities that did not stop the universe being built."""
        return loads(_types.ValidationReport, self._native.warnings()).issues

    def __len__(self) -> int:
        return len(self.ids)

    def screen(
        self,
        *,
        as_of: _arrays.DateLike | None = None,
        max_carry_days: int = 5,
        measures: Sequence[_types.Measure] | None = None,
        rules: Sequence[RuleLike] = (),
        sort: SortLike | None = None,
        limit: int | None = None,
        formulas: Mapping[str, str] | None = None,
        name: str | None = None,
    ) -> _types.Screen:
        """Screens the universe: each security measured on its own closes up to `as_of`, judged
        on the `rules`, with the matches sorted and limited. The raw results are in
        ``screen.results``, one row per security, and the screening report in
        ``screen.report``, which ``print(screen)`` prints and ``screen.show()`` shows; `name`
        titles it. :func:`finalytics.pandas.securities` turns the results into a DataFrame.

        - `as_of`: the date measured to; the latest date in the universe by default. A security
          whose last close is more than `max_carry_days` (5) earlier is stale: listed, but not
          measured.
        - `measures`: the measures to compute, such as ``["return_1y", "rsi_14"]``; every one
          the universe allows by default. Those the rules and the sort use are computed too.
        - `rules`: all of which a match meets, each ``(name, op, value)``: a measure or numeric
          field compared with ``"<"``, ``"<="``, ``">"`` or ``">="`` a number, or
          ``"between"`` a ``(low, high)`` pair; or a text field ``"in"`` or ``"not_in"`` a list
          of values. A security without the value a rule tests fails it.
        - `sort`: ``(name, order)``, a measure or numeric field and ``"descending"`` or
          ``"ascending"``; `limit`: the most matches to keep.
        - `formulas`: numeric fields computed for each security, by name, in order, such as
          ``{"risk_adjusted": "return_1y / volatility"}``, which the rules, the sort and the
          results then take by name. A formula is numbers, the names of measures, numeric
          fields and earlier formulas, ``+ - * /``, parentheses, and ``abs``, ``sqrt``, ``log``,
          ``min`` and ``max``; a security has no value where an input has none or the formula
          has no finite result, such as on division by zero.

        Every problem is reported together, in one :class:`~finalytics.ValidationError`.
        """
        options = _types.ScreenOptions(
            as_of=None if as_of is None else _arrays.date(as_of),
            max_carry_days=int(max_carry_days),
            measures=None if measures is None else [str(m) for m in measures],
            rules=[_rule(r, k) for k, r in enumerate(rules)],
            sort=_sort(sort),
            limit=None if limit is None else int(limit),
            formulas={str(k): str(v) for k, v in (formulas or {}).items()},
        )
        result = loads(_types.Screen, call(self._native.screen, dumps(options, "options")))
        if name is not None:
            result = dataclasses.replace(result, report=dataclasses.replace(result.report, name=name))
        return result

    def __repr__(self) -> str:
        benchmark = f", against {self.benchmark}" if self.benchmark else ""
        return f"Universe({len(self)} securities{benchmark})"
