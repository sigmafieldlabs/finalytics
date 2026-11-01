"""Risk analytics on returns: statistics, drawdowns, value at risk and expected shortfall,
covariance estimation and risk decomposition.

Every function takes returns as any one-dimensional sequence of numbers (a list, a NumPy array or
a pandas Series), one return per period::

    import finalytics as fl

    stats = fl.risk.statistics(returns, periods_per_year=252, risk_free_rate=0.03)
    tail = fl.risk.tail_risk(returns, confidence=0.99, method="cornish_fisher")
"""

from __future__ import annotations

import json
from collections import defaultdict
from collections.abc import Callable, Sequence
from typing import Any

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._arrays import FloatArray
from finalytics._convert import dumps, loads
from finalytics._errors import call
from finalytics._types import (
    CovarianceEstimate,
    CovarianceMethod,
    Drawdown,
    Ewma,
    GroupRisk,
    LedoitWolf,
    Matrix,
    Oas,
    RelativeStatistics,
    RiskDecomposition,
    Sample,
    Statistics,
    StatisticsOptions,
    TailMethod,
    TailRisk,
)

__all__ = [
    "CovarianceEstimate",
    "CovarianceMethod",
    "Drawdown",
    "Ewma",
    "GroupRisk",
    "LedoitWolf",
    "Matrix",
    "Oas",
    "RelativeStatistics",
    "RiskDecomposition",
    "Sample",
    "Statistics",
    "StatisticsOptions",
    "TailMethod",
    "TailRisk",
    "correlation",
    "covariance",
    "drawdown_series",
    "drawdowns",
    "group_risk",
    "matrix",
    "normal_cdf",
    "normal_pdf",
    "normal_quantile",
    "relative_statistics",
    "risk_decomposition",
    "rolling_relative_statistics",
    "rolling_statistics",
    "rolling_tail_risk",
    "statistics",
    "tail_risk",
    "to_array",
    "tracking_error_decomposition",
]


def _options(periods_per_year: float, risk_free_rate: float, target_return: float) -> str:
    return dumps(
        StatisticsOptions(
            periods_per_year=float(periods_per_year),
            risk_free_rate=float(risk_free_rate),
            target_return=float(target_return),
        )
    )


def _returns(values: Any) -> bytes:
    array = np.asarray(values, dtype=np.float64)
    if array.ndim != 1:
        raise ValueError("returns must be one-dimensional")
    return _arrays.floats_to_buffer(array)


def statistics(
    returns: Any, *, periods_per_year: float = 252, risk_free_rate: float = 0.0, target_return: float = 0.0
) -> Statistics:
    """Statistics of a returns series of at least two periods.

    `risk_free_rate` (for the Sharpe ratio) and `target_return` (for downside deviation and the
    Sortino ratio) are annual rates, converted to rates per period by compounding.
    """
    text = call(_finalytics.statistics, _returns(returns), _options(periods_per_year, risk_free_rate, target_return))
    return loads(Statistics, text)


def relative_statistics(
    returns: Any,
    benchmark: Any,
    *,
    periods_per_year: float = 252,
    risk_free_rate: float = 0.0,
    target_return: float = 0.0,
) -> RelativeStatistics:
    """Statistics of a returns series against a benchmark's returns over the same periods:
    active return, tracking error, information ratio, beta, alpha and correlation."""
    options = _options(periods_per_year, risk_free_rate, target_return)
    text = call(_finalytics.relative_statistics, _returns(returns), _returns(benchmark), options)
    return loads(RelativeStatistics, text)


def tail_risk(returns: Any, confidence: float = 0.95, method: TailMethod = "historical") -> TailRisk:
    """Value at risk and expected shortfall over one period, as positive losses, estimated by the
    ``"historical"``, ``"parametric"`` (normal) or ``"cornish_fisher"`` method."""
    return loads(TailRisk, call(_finalytics.tail_risk, _returns(returns), float(confidence), method))


def rolling_statistics(
    returns: Any,
    window: int,
    *,
    periods_per_year: float = 252,
    risk_free_rate: float = 0.0,
    target_return: float = 0.0,
) -> list[Statistics]:
    """:func:`statistics` over every window of `window` returns. Entry ``k`` covers returns
    ``k`` to ``k + window - 1``."""
    options = _options(periods_per_year, risk_free_rate, target_return)
    return loads(list[Statistics], call(_finalytics.rolling_statistics, _returns(returns), int(window), options))


def rolling_relative_statistics(
    returns: Any,
    benchmark: Any,
    window: int,
    *,
    periods_per_year: float = 252,
    risk_free_rate: float = 0.0,
    target_return: float = 0.0,
) -> list[RelativeStatistics]:
    """:func:`relative_statistics` over every window of `window` returns."""
    options = _options(periods_per_year, risk_free_rate, target_return)
    text = call(_finalytics.rolling_relative_statistics, _returns(returns), _returns(benchmark), int(window), options)
    return loads(list[RelativeStatistics], text)


def rolling_tail_risk(
    returns: Any, window: int, confidence: float = 0.95, method: TailMethod = "historical"
) -> list[TailRisk]:
    """:func:`tail_risk` over every window of `window` returns."""
    text = call(_finalytics.rolling_tail_risk, _returns(returns), int(window), float(confidence), method)
    return loads(list[TailRisk], text)


def _levels(levels: Any, dates: Any) -> tuple[bytes, bytes]:
    if dates is None:
        if not hasattr(levels, "index"):
            raise TypeError("give the dates, or a pandas Series indexed by date")
        dates, levels = levels.index, levels.to_numpy()
    return _arrays.dates_to_buffer(dates), _arrays.floats_to_buffer(levels)


def drawdowns(levels: Any, dates: Any = None) -> list[Drawdown]:
    """Every drawdown in a series of levels (values or a growth index), in date order, with its
    peak, trough and recovery dates. Give `dates`, or levels as a pandas Series indexed by date.
    Sort by ``depth`` to list the worst first."""
    return loads(list[Drawdown], call(_finalytics.drawdowns, *_levels(levels, dates)))


def drawdown_series(levels: Any, dates: Any = None) -> FloatArray:
    """The fall below the running peak on each date, as a negative return (0 at a peak)."""
    return _arrays.floats_from_buffer(call(_finalytics.drawdown_series, *_levels(levels, dates)))


def _method(method: str | CovarianceMethod, decay: float) -> CovarianceMethod:
    if not isinstance(method, str):
        return method
    named = {"sample": Sample(), "ledoit_wolf": LedoitWolf(), "oas": Oas(), "ewma": Ewma(decay=decay)}
    if method not in named:
        raise ValueError(f"unknown covariance method {method!r}: use sample, ewma, ledoit_wolf or oas")
    return named[method]


def covariance(
    returns: Any,
    labels: Sequence[str] | None = None,
    *,
    method: str | CovarianceMethod = "sample",
    decay: float = 0.94,
) -> CovarianceEstimate:
    """Estimates the covariance matrix of assets' returns, given as a two-dimensional array with
    one row per period and one column per asset, or as a pandas DataFrame (whose columns give the
    labels).

    `method` is ``"sample"``, ``"ewma"`` (with `decay`), ``"ledoit_wolf"`` or ``"oas"``. The
    matrix is per period: scale it to annualise, for example ``scaled(estimate.matrix, 252)``.
    """
    if labels is None and hasattr(returns, "columns"):
        labels = [str(c) for c in returns.columns]
    array = np.asarray(returns, dtype=np.float64)
    if array.ndim != 2:
        raise ValueError("returns must be two-dimensional: one row per period, one column per asset")
    if labels is None:
        labels = [f"asset {i}" for i in range(array.shape[1])]
    columns = _arrays.floats_to_buffer(array.T.copy())
    text = call(_finalytics.covariance, columns, json.dumps(list(labels)), dumps(_method(method, decay)))
    return loads(CovarianceEstimate, text)


def matrix(values: Any, labels: Sequence[str] | None = None) -> Matrix:
    """A labelled matrix from a square array or a pandas DataFrame."""
    if labels is None and hasattr(values, "columns"):
        labels = [str(c) for c in values.columns]
    array = np.asarray(values, dtype=np.float64)
    if array.ndim != 2 or array.shape[0] != array.shape[1]:
        raise ValueError("a matrix must be square")
    if labels is None:
        labels = [f"asset {i}" for i in range(array.shape[0])]
    return Matrix(labels=list(labels), values=array.ravel().tolist())


def to_array(value: Matrix) -> FloatArray:
    """A labelled matrix as a square NumPy array."""
    n = len(value.labels)
    return np.asarray(value.values, dtype=np.float64).reshape(n, n)


def scaled(value: Matrix, factor: float) -> Matrix:
    """Every entry multiplied by `factor`, for example 252 to annualise a daily covariance."""
    return Matrix(labels=list(value.labels), values=[v * factor for v in value.values])


def correlation(covariance: Matrix | CovarianceEstimate) -> Matrix:
    """The correlation matrix of a covariance matrix. Every variance must be greater than zero."""
    return loads(Matrix, call(_finalytics.correlation, dumps(_matrix(covariance))))


def _matrix(value: Matrix | CovarianceEstimate | Any) -> Matrix:
    if isinstance(value, CovarianceEstimate):
        return value.matrix
    if isinstance(value, Matrix):
        return value
    return matrix(value)


def risk_decomposition(weights: Any, covariance: Matrix | CovarianceEstimate | Any) -> RiskDecomposition:
    """Splits the volatility of a portfolio with `weights` into each asset's marginal and
    component risk; the components add up to the volatility."""
    text = call(_finalytics.risk_decomposition, _returns(weights), dumps(_matrix(covariance)))
    return loads(RiskDecomposition, text)


def tracking_error_decomposition(
    weights: Any, benchmark: Any, covariance: Matrix | CovarianceEstimate | Any
) -> RiskDecomposition:
    """The ex-ante tracking error against `benchmark` weights, split into each asset's
    contribution."""
    text = call(
        _finalytics.tracking_error_decomposition, _returns(weights), _returns(benchmark), dumps(_matrix(covariance))
    )
    return loads(RiskDecomposition, text)


def group_risk(decomposition: RiskDecomposition, group_of: Callable[[int, str], str]) -> list[GroupRisk]:
    """Adds up a decomposition by group, naming each asset's group from its position and label
    with `group_of`. Groups are returned in name order."""
    totals: dict[str, list[Any]] = defaultdict(lambda: [0.0, 0.0, 0.0])
    has_shares = True
    for k, c in enumerate(decomposition.contributions):
        entry = totals[group_of(k, c.label)]
        entry[0] += c.weight
        entry[1] += c.component
        if c.share is None:
            has_shares = False
        else:
            entry[2] += c.share
    return [
        GroupRisk(group=g, weight=v[0], component=v[1], share=v[2] if has_shares else None)
        for g, v in sorted(totals.items())
    ]


def normal_cdf(x: float) -> float:
    """The standard normal distribution function."""
    return _finalytics.normal_cdf(float(x))


def normal_pdf(x: float) -> float:
    """The standard normal density."""
    return _finalytics.normal_pdf(float(x))


def normal_quantile(p: float) -> float:
    """The standard normal quantile (inverse distribution function)."""
    return _finalytics.normal_quantile(float(p))



