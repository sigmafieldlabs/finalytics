"""Assets: the prices or returns of things one might hold, and what can be learnt from them."""

from __future__ import annotations

import dataclasses
import datetime
import json
import math
import os
from collections.abc import Mapping, Sequence
from typing import Any

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._arrays import DateArray
from finalytics._convert import dumps, loads
from finalytics._errors import call

__all__ = ["Assets"]

BoundLike = _types.Bound | tuple[float, float]
"""Limits on a weight: a :class:`~finalytics.types.Bound` or a ``(min, max)`` pair."""

GroupLike = _types.AssetGroup | tuple[Sequence[str], float, float]
"""A group of assets and the limits on their total weight: a
:class:`~finalytics.types.AssetGroup` or an ``(assets, min, max)`` triple."""


def _covariance(method: str | _types.CovarianceMethod) -> str:
    from finalytics.risk import _method

    return dumps(_method(method, 0.94))


def _price_table(prices: Any) -> _types.PriceTable:
    """Prices as a PriceTable: a pandas DataFrame indexed by date with one column per asset, or a
    PriceTable."""
    if isinstance(prices, _types.PriceTable):
        return prices
    if hasattr(prices, "columns") and hasattr(prices, "index"):
        values = np.asarray(prices.to_numpy(dtype=np.float64, na_value=np.nan), dtype=np.float64)
        columns = [[None if math.isnan(v) else float(v) for v in values[:, j]] for j in range(values.shape[1])]
        return _types.PriceTable(
            dates=[datetime.date.fromisoformat(_arrays.date(d)) for d in prices.index],
            labels=[str(c) for c in prices.columns],
            columns=columns,
        )
    raise TypeError(
        "prices must be a pandas DataFrame (indexed by date, one column per asset), a PriceTable, "
        "or a CSV file or text"
    )


def _bound(value: BoundLike) -> _types.Bound:
    if isinstance(value, _types.Bound):
        return value
    low, high = value
    return _types.Bound(min=float(low), max=float(high))


def _group(value: GroupLike) -> _types.AssetGroup:
    if isinstance(value, _types.AssetGroup):
        return value
    assets, low, high = value
    return _types.AssetGroup(assets=list(assets), min=float(low), max=float(high))


class Assets:
    """The prices or returns of several assets over the same periods, with the settings that
    turn them into annual figures.

    `prices` is a pandas DataFrame indexed by date with one column of prices per asset, a
    :class:`~finalytics.types.PriceTable`, or a CSV file (a ``date`` column and one column per
    asset) given as a path or as text. Dates on which an asset has no price are left out, with
    a warning, so that every asset's returns cover the same periods.

    - `risk_free_rate`: the annual risk-free rate, for Sharpe ratios; 0 by default.
    - `risk_free_asset`: one of the assets, such as T-bills, to serve as the risk-free asset
      instead: its mean return is the risk-free rate, and optimisation leaves it out.
    - `periods_per_year`: 252 for daily returns, 52 for weekly, 12 for monthly; detected from
      the dates by default.
    - `covariance`: ``"ledoit_wolf"`` (the default), ``"sample"``, ``"oas"`` or ``"ewma"``.

    ::

        import finalytics as fl

        assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
        result = assets.optimise(objective="maximum_sharpe", bounds=(0.0, 0.4))
        result.weights
    """

    def __init__(
        self,
        prices: Any,
        *,
        risk_free_rate: float = 0.0,
        risk_free_asset: str | None = None,
        periods_per_year: float | None = None,
        covariance: str | _types.CovarianceMethod = "ledoit_wolf",
    ):
        if isinstance(prices, (str, os.PathLike)):
            from finalytics.portfolio import _text

            native = call(_finalytics.NativeAssets.from_csv, _text(prices))
        else:
            native = call(_finalytics.NativeAssets.from_prices, dumps(_price_table(prices)))
        self._native = self._configure(native, risk_free_rate, risk_free_asset, periods_per_year, covariance)

    @staticmethod
    def _configure(
        native: Any,
        risk_free_rate: float,
        risk_free_asset: str | None,
        periods_per_year: float | None,
        covariance: str | _types.CovarianceMethod,
    ) -> Any:
        if periods_per_year is not None:
            native = native.with_periods_per_year(float(periods_per_year))
        native = call(native.with_covariance, _covariance(covariance))
        if risk_free_asset is not None:
            if risk_free_rate:
                raise ValueError("give a risk_free_rate or a risk_free_asset, not both")
            return call(native.with_risk_free_asset, risk_free_asset)
        return native.with_risk_free_rate(float(risk_free_rate))

    @classmethod
    def _wrap(cls, native: Any) -> Assets:
        assets = cls.__new__(cls)
        assets._native = native
        return assets

    @classmethod
    def from_returns(
        cls,
        returns: Any,
        *,
        dates: Any = None,
        labels: Sequence[str] | None = None,
        risk_free_rate: float = 0.0,
        risk_free_asset: str | None = None,
        periods_per_year: float | None = None,
        covariance: str | _types.CovarianceMethod = "ledoit_wolf",
    ) -> Assets:
        """Assets from their returns: a pandas DataFrame indexed by the date each return ends on,
        with one column per asset, or a two-dimensional array (one row per period) with `dates`
        and `labels`. The other settings are as for :class:`Assets`."""
        if hasattr(returns, "columns") and hasattr(returns, "index"):
            labels = [str(c) for c in returns.columns] if labels is None else labels
            dates = returns.index if dates is None else dates
        array = np.asarray(returns, dtype=np.float64)
        if array.ndim != 2:
            raise ValueError("returns must be two-dimensional: one row per period, one column per asset")
        if dates is None or labels is None:
            raise ValueError("give the dates and labels of the returns, or a DataFrame")
        native = call(
            _finalytics.NativeAssets.from_returns,
            _arrays.dates_to_buffer(dates),
            json.dumps(list(labels)),
            [[float(v) for v in array[:, j]] for j in range(array.shape[1])],
        )
        assets = cls._wrap(native)
        assets._native = cls._configure(native, risk_free_rate, risk_free_asset, periods_per_year, covariance)
        return assets

    @property
    def labels(self) -> list[str]:
        """The assets."""
        return self._native.labels()

    @property
    def dates(self) -> DateArray:
        """The date each return ends on."""
        return _arrays.dates_from_buffer(self._native.dates())

    @property
    def returns(self) -> np.ndarray:
        """The returns: one row per period, one column per asset."""
        columns = [_arrays.floats_from_buffer(c) for c in self._native.returns()]
        return np.column_stack(columns)

    @property
    def periods_per_year(self) -> float:
        """The number of return periods in a year."""
        return self._native.periods_per_year()

    @property
    def risk_free_rate(self) -> float:
        """The annual risk-free rate."""
        return self._native.risk_free_rate()

    @property
    def risk_free_asset(self) -> str | None:
        """The risk-free asset, if one of the assets is."""
        return self._native.risk_free_asset()

    @property
    def warnings(self) -> list[_types.Issue]:
        """Problems met reading the prices that did not stop the assets being built, such as
        dates left out because an asset had no price."""
        return loads(_types.ValidationReport, self._native.warnings()).issues

    def statistics(self) -> list[_types.AssetStatistics]:
        """Each asset's statistics (annualised return and volatility, Sharpe and Sortino
        ratios, drawdown and more), with its value at risk and expected shortfall over one
        period at 95%, in the order of the assets."""
        return loads(list[_types.AssetStatistics], call(self._native.statistics))

    def covariance(self) -> _types.CovarianceEstimate:
        """The covariance of the assets' returns, annualised."""
        return loads(_types.CovarianceEstimate, call(self._native.covariance))

    def correlation(self) -> _types.Matrix:
        """The correlation of the assets' returns."""
        return loads(_types.Matrix, call(self._native.correlation))

    def mean_returns(self) -> dict[str, float]:
        """Each asset's historical mean return, annualised: the expected returns optimisation
        uses unless others are given."""
        return dict(zip(self.labels, self._native.mean_returns(), strict=True))

    def optimise(
        self,
        *,
        objective: _types.ObjectiveName = "maximum_sharpe",
        risk_free_rate: float | None = None,
        target: float | None = None,
        risk_aversion: float | None = None,
        uncertainty: _types.ReturnUncertainty | None = None,
        confidence: float | None = None,
        budgets: Mapping[str, float] | None = None,
        linkage: _types.Linkage | None = None,
        bounds: BoundLike | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        expected_returns: Mapping[str, float] | None = None,
    ) -> _types.OptimisationResult:
        """The weights of the risky assets that best meet `objective`, fully invested:

        - ``"maximum_sharpe"`` (the default), ``"minimum_variance"``,
          ``"maximum_diversification"`` and ``"hierarchical_risk_parity"`` (with `linkage`);
        - ``"mean_variance"`` and ``"robust_mean_variance"``, with `risk_aversion` (and
          `uncertainty`, a :class:`~finalytics.types.ReturnUncertainty`);
        - ``"target_return"`` and ``"target_volatility"``, with an annual `target`;
        - ``"minimum_cvar"``, at `confidence` (0.95 by default);
        - ``"risk_budgeting"``, with `budgets` by asset (equal shares by default).

        `bounds` limits every weight, ``(0, 1)`` by default; `asset_bounds` limits particular
        assets, such as ``{"Gold": (0, 0.1)}``; `groups` limits the total weight of groups of
        assets, such as ``{"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)}``. Expected
        returns are the annualised historical means unless `expected_returns` gives others.
        """
        options = _types.OptimiseOptions(
            objective=objective,
            risk_free_rate=risk_free_rate,
            target=target,
            risk_aversion=risk_aversion,
            uncertainty=uncertainty,
            confidence=confidence,
            budgets=None if budgets is None else {k: float(v) for k, v in budgets.items()},
            linkage=linkage,
            bounds=None if bounds is None else _bound(bounds),
            asset_bounds={k: _bound(v) for k, v in (asset_bounds or {}).items()},
            groups={k: _group(v) for k, v in (groups or {}).items()},
            expected_returns={k: float(v) for k, v in (expected_returns or {}).items()},
        )
        result = loads(_types.OptimisationResult, call(self._native.optimise, dumps(options)))
        if result.weights is not None:
            # In the order of the assets, rather than of their names.
            order = {label: i for i, label in enumerate(self.labels)}
            weights = dict(sorted(result.weights.items(), key=lambda item: order[item[0]]))
            result = dataclasses.replace(result, weights=weights)
        return result

    def frontier(
        self,
        *,
        points: int = 20,
        bounds: BoundLike | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        expected_returns: Mapping[str, float] | None = None,
    ) -> list[_types.FrontierPoint]:
        """The efficient frontier: `points` portfolios of the risky assets, from the one with
        the least variance to the one with the largest expected return, under the limits given
        as for :meth:`optimise`."""
        options = _types.OptimiseOptions(
            bounds=None if bounds is None else _bound(bounds),
            asset_bounds={k: _bound(v) for k, v in (asset_bounds or {}).items()},
            groups={k: _group(v) for k, v in (groups or {}).items()},
            expected_returns={k: float(v) for k, v in (expected_returns or {}).items()},
        )
        return loads(list[_types.FrontierPoint], call(self._native.frontier, int(points), dumps(options)))

    def __repr__(self) -> str:
        return f"Assets({len(self.labels)} assets, {len(self.dates)} periods, {self.periods_per_year:g} a year)"
