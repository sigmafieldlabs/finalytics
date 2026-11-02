"""Performance measurement: time- and money-weighted returns, periods and contribution."""

from __future__ import annotations

import datetime
import json
from collections import defaultdict
from collections.abc import Callable, Iterable
from typing import Any

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._arrays import DateArray, DateLike, FloatArray
from finalytics._convert import dumps, loads
from finalytics._errors import call

__all__ = [
    "Performance",
    "PeriodLike",
    "attribution_period",
    "brinson_fachler",
    "group_contributions",
    "karnosky_singer",
    "period",
]

PeriodLike = str | _types.Period
"""A reporting period: ``"MTD"``, ``"QTD"``, ``"YTD"``, ``"ITD"`` (since inception), ``"1Y"``,
``"3Y"`` and so on, or a :data:`~finalytics.types.Period`."""


def period(value: PeriodLike) -> _types.Period:
    """A reporting period from its short label: ``"MTD"``, ``"QTD"``, ``"YTD"``, ``"ITD"``,
    a number of years such as ``"3Y"``, or ``"since 2026-01-31"``."""
    if not isinstance(value, str):
        return value
    return loads(_types.Period, call(_finalytics.parse_period, value))


class Performance:
    """A portfolio's daily time-weighted returns, chain-linked from its values and external
    flows, and the measures built on them."""

    _native: Any

    @classmethod
    def _wrap(cls, native: Any) -> Performance:
        found = cls.__new__(cls)
        found._native = native
        return found

    @classmethod
    def from_values(
        cls,
        dates: Any,
        values: Any,
        flows: Any = None,
        *,
        timing: _types.FlowTiming = "start_of_day",
    ) -> Performance:
        """Performance from end-of-day values and the external flows since the previous date
        (positive into the portfolio; none by default)."""
        values = np.asarray(values, dtype=np.float64)
        flows = np.zeros_like(values) if flows is None else np.asarray(flows, dtype=np.float64)
        native = call(
            _finalytics.NativePerformance.from_values,
            _arrays.dates_to_buffer(dates),
            _arrays.floats_to_buffer(values),
            _arrays.floats_to_buffer(flows),
            timing,
        )
        return cls._wrap(native)

    @property
    def dates(self) -> DateArray:
        """The valuation dates."""
        return _arrays.dates_from_buffer(self._native.dates())

    @property
    def returns(self) -> FloatArray:
        """The daily returns, one per date after the first (``returns[k]`` ends on
        ``dates[k + 1]``)."""
        return _arrays.floats_from_buffer(self._native.returns())

    @property
    def growth(self) -> FloatArray:
        """The growth of 1 invested on the first date, one value per date."""
        return _arrays.floats_from_buffer(self._native.growth())

    @property
    def values(self) -> FloatArray:
        """The portfolio's value on each date, in the base currency."""
        return _arrays.floats_from_buffer(self._native.values())

    @property
    def warnings(self) -> list[_types.Issue]:
        """Prices and rates carried forward while valuing the portfolio."""
        return loads(_types.ValidationReport, self._native.warnings()).issues

    def _range(self, start: DateLike | None, end: DateLike | None) -> tuple[str, str]:
        dates = self.dates
        if len(dates) == 0:
            raise ValueError("the performance has no dates")
        return (
            _arrays.date(dates[0] if start is None else start),
            _arrays.date(dates[-1] if end is None else end),
        )

    def time_weighted(self, start: DateLike | None = None, end: DateLike | None = None) -> float | None:
        """The time-weighted return from the valuation on or before `start` to the one on or
        before `end` (by default, the whole history), not annualised."""
        return call(self._native.time_weighted, *self._range(start, end))

    def period(self, which: PeriodLike, as_of: DateLike | None = None) -> _types.PeriodReturn | None:
        """The return over a reporting period ending on `as_of` (by default the last valuation),
        annualised when the period is a year or longer. None when the history does not reach
        back to the start of the period."""
        text = call(self._native.period, dumps(period(which)), _arrays.optional_date(as_of))
        return None if text is None else loads(_types.PeriodReturn, text)

    def periods(
        self, which: Iterable[PeriodLike] = ("MTD", "QTD", "YTD", "1Y", "3Y", "5Y", "ITD"), as_of: DateLike | None = None
    ) -> dict[str, _types.PeriodReturn | None]:
        """The returns over several periods, keyed by their labels."""
        out = {}
        for p in which:
            label = p if isinstance(p, str) else repr(p)
            out[label] = self.period(p, as_of)
        return out

    def money_weighted(self, start: DateLike | None = None, end: DateLike | None = None) -> float | None:
        """The money-weighted return (internal rate of return, annualised, actual/365), or None
        when it cannot be found."""
        return call(self._native.money_weighted, *self._range(start, end))

    def modified_dietz(self, start: DateLike | None = None, end: DateLike | None = None) -> float | None:
        """The Modified Dietz return, which weights each flow by the time it was invested."""
        return call(self._native.modified_dietz, *self._range(start, end))

    def excess_return(
        self,
        benchmark: Any,
        which: PeriodLike = "ITD",
        as_of: DateLike | None = None,
        *,
        dates: Any = None,
        max_carry_days: int = 5,
    ) -> _types.ExcessReturn | None:
        """The portfolio's return over a period against a benchmark's, given as levels (an index
        or prices): a pandas Series indexed by date, or `dates` with the levels."""
        if dates is None:
            if not hasattr(benchmark, "index"):
                raise TypeError("give the benchmark's dates, or a pandas Series indexed by date")
            dates, benchmark = benchmark.index, benchmark.to_numpy()
        text = call(
            self._native.excess_return,
            dumps(period(which)),
            _arrays.optional_date(as_of),
            _arrays.dates_to_buffer(dates),
            _arrays.floats_to_buffer(benchmark),
            max_carry_days,
        )
        return None if text is None else loads(_types.ExcessReturn, text)

    def contributions(self, start: DateLike | None = None, end: DateLike | None = None) -> list[_types.Contribution]:
        """Each holding's contribution to the time-weighted return from `start` to `end`; with
        :meth:`expenses`, the contributions add up to the return. Needs performance measured from
        a portfolio."""
        return loads(list[_types.Contribution], call(self._native.contributions, *self._range(start, end)))

    def expenses(self, start: DateLike | None = None, end: DateLike | None = None) -> float:
        """What fees and taxes charged to the accounts rather than to a holding took from the
        time-weighted return from `start` to `end`: negative, or zero without them. Needs
        performance measured from a portfolio."""
        return call(self._native.expenses, *self._range(start, end))

    def __repr__(self) -> str:
        dates = self.dates
        span = f"{dates[0]} to {dates[-1]}" if len(dates) else "empty"
        return f"Performance({span}, {len(dates)} valuations)"


def group_contributions(
    contributions: Iterable[_types.Contribution], group_of: Callable[[_types.HoldingKey], str]
) -> list[_types.GroupContribution]:
    """Adds up contributions by group, naming each holding's group with `group_of`. Groups are
    returned in name order."""
    totals: dict[str, list[float]] = defaultdict(lambda: [0.0, 0.0])
    for c in contributions:
        entry = totals[group_of(c.holding)]
        entry[0] += c.contribution
        entry[1] += c.average_weight
    return [
        _types.GroupContribution(group=g, contribution=v[0], average_weight=v[1]) for g, v in sorted(totals.items())
    ]


_SEGMENT_FIELDS = ("portfolio_weight", "portfolio_return", "benchmark_weight", "benchmark_return")


def attribution_period(start: DateLike, end: DateLike, segments: Any) -> _types.AttributionPeriod:
    """A period for :func:`brinson_fachler` from its dates and segments: a pandas DataFrame
    indexed by segment name with the columns ``portfolio_weight``, ``portfolio_return``,
    ``benchmark_weight`` and ``benchmark_return``, or a sequence of
    :class:`~finalytics.types.Segment`."""
    if hasattr(segments, "columns"):
        missing = [c for c in _SEGMENT_FIELDS if c not in segments.columns]
        if missing:
            raise ValueError(f"segments needs the columns {', '.join(missing)}")
        segments = [
            _types.Segment(name=str(name), **{c: float(row[c]) for c in _SEGMENT_FIELDS})
            for name, row in segments.iterrows()
        ]
    return _types.AttributionPeriod(
        start=datetime.date.fromisoformat(_arrays.date(start)),
        end=datetime.date.fromisoformat(_arrays.date(end)),
        segments=list(segments),
    )


def brinson_fachler(
    periods: Iterable[_types.AttributionPeriod], linking: _types.Linking = "carino"
) -> _types.Attribution:
    """Brinson–Fachler attribution of the excess return over the benchmark, by segment:
    allocation, selection and interaction in each period, linked over the periods by Cariño's or
    Menchero's coefficients so that they add up to the excess of the compounded returns."""
    text = call(_finalytics.brinson_fachler, dumps(list(periods)), str(linking))
    return loads(_types.Attribution, text)


def karnosky_singer(
    periods: Iterable[_types.CurrencyAttributionPeriod], linking: _types.Linking = "carino"
) -> _types.CurrencyAttribution:
    """Currency attribution by the method of Karnosky and Singer: market effects on each
    segment's return in excess of its currency's cash return, currency effects on each currency's
    return including its interest rate, and the residual the additive model leaves out, linked
    over the periods."""
    text = call(_finalytics.karnosky_singer, dumps(list(periods)), str(linking))
    return loads(_types.CurrencyAttribution, text)
