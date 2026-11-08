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
from finalytics._errors import call, invalid

__all__ = ["Assets", "methods"]


def methods() -> list[_types.MethodInfo]:
    """Every optimisation method with what it takes: its parameters, with their defaults and the
    values allowed, the limits on weights it accepts, and whether it resamples or is measured on
    the periods' returns. :meth:`Assets.optimise` checks its keywords against the same
    descriptions, so a form built from them offers only what the method uses.

    ::

        cvar = next(m for m in fl.methods() if m.method == "minimum_cvar")
        [p.name for p in cvar.parameters]   # ['confidence', 'target', 'risk_free_rate', ...]
    """
    return loads(list[_types.MethodInfo], _finalytics.methods())

BoundLike = _types.Bound | tuple[float, float]
"""Limits on a weight: a :class:`~finalytics.types.Bound` or a ``(min, max)`` pair."""

MixLike = Any
"""An investment mix: weights by asset, a ``(name, weights)`` pair, a
:class:`~finalytics.FixedMix` or :class:`~finalytics.GlidePath`, or a result with weights."""

FlowsLike = Mapping[Any, float] | Sequence[tuple[Any, float]]
"""Money paid in or taken out on dates: amounts by date, such as ``{"2020-01-31": 500}``, or
``(date, amount)`` pairs; a date is a :class:`datetime.date` or its ISO text."""

GroupLike = _types.AssetGroup | tuple[Sequence[str] | str, float, float]
"""A group of assets and the limits on their total weight: a
:class:`~finalytics.types.AssetGroup`, an ``(assets, min, max)`` triple, or a
``(classification, min, max)`` triple, such as ``("asset_class", 0, 0.6)`` for the group named
``"Equity"``: the assets classed as the group's name under the classification."""


def _covariance(method: str | _types.CovarianceMethod) -> str:
    from finalytics.risk import _method

    return dumps(_method(method, 0.94, "covariance"))


def _mean(method: str | _types.MeanMethod) -> str:
    from finalytics.risk import _mean

    return dumps(_mean(method))


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
    if isinstance(assets, str):
        return _types.AssetGroup(classification=assets, min=float(low), max=float(high))
    return _types.AssetGroup(assets=list(assets), min=float(low), max=float(high))


class Assets:
    """The prices or returns of several assets over the same periods, with the settings that
    turn them into annual figures.

    `prices` is a pandas DataFrame indexed by date with one column of prices per asset, a
    :class:`~finalytics.types.PriceTable`, or a CSV file (a ``date`` column and one column per
    asset) given as a path or as text. Prices should include income, as adjusted closes do.
    Dates on which an asset has no price are left out, with a warning, so that every asset's
    returns cover the same periods.

    - `risk_free_rate`: the annual risk-free rate, for Sharpe ratios; 0 by default.
    - `risk_free_asset`: one of the assets, such as T-bills, to serve as the risk-free asset
      instead: its mean return is the risk-free rate, and optimisation leaves it out.
    - `periods_per_year`: 252 for daily returns, 52 for weekly, 12 for monthly; detected from
      the dates by default.
    - `covariance`: ``"ledoit_wolf"`` (the default), ``"sample"``, ``"oas"``, ``"ewma"``
      (with a decay of 0.94), ``"constant_correlation"``, ``"denoised"`` or
      ``"nonlinear_shrinkage"``, or a :class:`~finalytics.types.CovarianceMethod` such as
      ``Ewma(decay=0.97)`` or ``Statistical(factors=3)``.
    - `mean_returns`: how expected returns are estimated: ``"historical"`` (the default),
      ``"james_stein"`` (shrunk towards the assets' average) or ``"ewma"`` (exponentially
      weighted, with a decay of 0.94), or a :class:`~finalytics.types.MeanMethod` such as
      ``EwmaMean(decay=0.97)``.
    - `classes`: each asset's class under classification schemes, such as
      ``{"asset_class": {"US Equity": "Equity", "Bonds": "Fixed income"}}``, so that group limits
      can name a class: ``groups={"Equity": ("asset_class", 0, 0.6)}``. A portfolio's assets
      carry its instruments' classes.

    ::

        import finalytics as fl

        assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
        result = assets.optimise(method="maximum_sharpe", max_weight=0.4)
        result.results.weights
    """

    def __init__(
        self,
        prices: Any,
        *,
        risk_free_rate: float | None = None,
        risk_free_asset: str | None = None,
        periods_per_year: float | None = None,
        covariance: str | _types.CovarianceMethod = "ledoit_wolf",
        mean_returns: str | _types.MeanMethod = "historical",
        classes: Mapping[str, Mapping[str, str]] | None = None,
    ):
        if isinstance(prices, (str, os.PathLike)):
            from finalytics.portfolio import _text

            native = call(_finalytics.NativeAssets.from_csv, _text(prices))
        else:
            native = call(_finalytics.NativeAssets.from_prices, dumps(_price_table(prices)))
        self._native = self._configure(
            native, risk_free_rate, risk_free_asset, periods_per_year, covariance, mean_returns, classes
        )

    @staticmethod
    def _configure(
        native: Any,
        risk_free_rate: float | None,
        risk_free_asset: str | None,
        periods_per_year: float | None,
        covariance: str | _types.CovarianceMethod,
        mean_returns: str | _types.MeanMethod,
        classes: Mapping[str, Mapping[str, str]] | None = None,
    ) -> Any:
        settings = {
            "risk_free_rate": None if risk_free_rate is None else float(risk_free_rate),
            "risk_free_asset": risk_free_asset,
            "periods_per_year": None if periods_per_year is None else float(periods_per_year),
            "covariance": json.loads(_covariance(covariance)),
            "mean_returns": json.loads(_mean(mean_returns)),
            "classes": None
            if classes is None
            else {str(k): {str(a): str(c) for a, c in v.items()} for k, v in classes.items()},
        }
        return call(native.with_settings, dumps(settings))

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
        risk_free_rate: float | None = None,
        risk_free_asset: str | None = None,
        periods_per_year: float | None = None,
        covariance: str | _types.CovarianceMethod = "ledoit_wolf",
        mean_returns: str | _types.MeanMethod = "historical",
        classes: Mapping[str, Mapping[str, str]] | None = None,
    ) -> Assets:
        """Assets from their returns: a pandas DataFrame indexed by the date each return ends on,
        with one column per asset, or a two-dimensional array (one row per period) with `dates`
        and `labels`. The other settings are as for :class:`Assets`."""
        if hasattr(returns, "columns") and hasattr(returns, "index"):
            labels = [str(c) for c in returns.columns] if labels is None else labels
            dates = returns.index if dates is None else dates
        array = np.asarray(returns, dtype=np.float64)
        if array.ndim != 2:
            raise invalid(("returns", "must be two-dimensional: one row per period, one column per asset"))
        if dates is None or labels is None:
            raise invalid(("dates", "give the dates and labels of the returns, or a DataFrame"))
        native = call(
            _finalytics.NativeAssets.from_returns,
            _arrays.dates_to_buffer(dates),
            json.dumps(list(labels)),
            [[float(v) for v in array[:, j]] for j in range(array.shape[1])],
        )
        assets = cls._wrap(native)
        assets._native = cls._configure(
            native, risk_free_rate, risk_free_asset, periods_per_year, covariance, mean_returns, classes
        )
        return assets

    @property
    def labels(self) -> list[str]:
        """The assets."""
        return self._native.labels()

    @property
    def classes(self) -> dict[str, dict[str, str]]:
        """Each classification scheme's class of each asset it classifies."""
        return json.loads(self._native.classes())

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
    def risky_assets(self) -> list[str]:
        """The assets optimisation chooses among: all but the cash."""
        return self._native.risky_assets()

    @property
    def cash_assets(self) -> list[str]:
        """The cash among the assets, which optimisation does not choose: the risk-free asset,
        if any, then cash in other currencies, such as a portfolio's euro cash."""
        return self._native.cash_assets()

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
        """Each asset's mean return, annualised, by the `mean_returns` estimator (the risk-free
        asset's is its historical mean): the expected returns optimisation uses unless others
        are given."""
        return dict(zip(self.labels, self._native.mean_returns(), strict=True))

    def _options(
        self,
        method: str,
        *,
        risk_free_rate: float | None = None,
        target: float | None = None,
        risk_aversion: float | None = None,
        uncertainty: _types.ReturnUncertainty | None = None,
        confidence: float | None = None,
        budgets: Mapping[str, float] | None = None,
        linkage: _types.Linkage | None = None,
        bounds: BoundLike | None = None,
        max_weight: float | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        min_effective_assets: float | None = None,
        expected_returns: Mapping[str, float] | None = None,
        resample: int | None = None,
        current_weights: Mapping[str, float] | None = None,
        max_turnover: float | None = None,
        min_trade: float | None = None,
        max_trades: int | None = None,
        benchmark_weights: Mapping[str, float] | None = None,
        max_tracking_error: float | None = None,
        cash: BoundLike | None = None,
    ) -> _types.OptimiseOptions:
        # Numbers JSON cannot hold, named by keyword: a comparison or backtest repeats the
        # options in each strategy.
        numbers = {
            "risk_free_rate": risk_free_rate,
            "target": target,
            "risk_aversion": risk_aversion,
            "confidence": confidence,
            "max_weight": max_weight,
            "min_effective_assets": min_effective_assets,
            "max_turnover": max_turnover,
            "min_trade": min_trade,
            "max_tracking_error": max_tracking_error,
        }
        numbers.update({f"budgets.{k}": v for k, v in (budgets or {}).items()})
        numbers.update({f"expected_returns.{k}": v for k, v in (expected_returns or {}).items()})
        numbers.update({f"current_weights.{k}": v for k, v in (current_weights or {}).items()})
        numbers.update({f"benchmark_weights.{k}": v for k, v in (benchmark_weights or {}).items()})
        bad = [(k, float(v)) for k, v in numbers.items() if v is not None and not math.isfinite(float(v))]
        if bad:
            raise invalid(*((f"options.{k}", f"must be a finite number, not {v}") for k, v in bad))
        return _types.OptimiseOptions(
            method=method,
            risk_free_rate=risk_free_rate,
            target=target,
            risk_aversion=risk_aversion,
            uncertainty=uncertainty,
            confidence=confidence,
            budgets=None if budgets is None else {k: float(v) for k, v in budgets.items()},
            linkage=linkage,
            bounds=None if bounds is None else _bound(bounds),
            max_weight=None if max_weight is None else float(max_weight),
            asset_bounds={k: _bound(v) for k, v in (asset_bounds or {}).items()},
            groups={k: _group(v) for k, v in (groups or {}).items()},
            min_effective_assets=None if min_effective_assets is None else float(min_effective_assets),
            expected_returns={k: float(v) for k, v in (expected_returns or {}).items()},
            resample=None if resample is None else int(resample),
            current_weights={k: float(v) for k, v in (current_weights or {}).items()},
            max_turnover=None if max_turnover is None else float(max_turnover),
            min_trade=None if min_trade is None else float(min_trade),
            max_trades=None if max_trades is None else int(max_trades),
            benchmark_weights={k: float(v) for k, v in (benchmark_weights or {}).items()},
            max_tracking_error=None if max_tracking_error is None else float(max_tracking_error),
            cash=None if cash is None else _bound(cash),
        )

    def optimise(
        self,
        *,
        method: _types.Method = "maximum_sharpe",
        risk_free_rate: float | None = None,
        target: float | None = None,
        risk_aversion: float | None = None,
        uncertainty: _types.ReturnUncertainty | None = None,
        confidence: float | None = None,
        budgets: Mapping[str, float] | None = None,
        linkage: _types.Linkage | None = None,
        bounds: BoundLike | None = None,
        max_weight: float | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        min_effective_assets: float | None = None,
        expected_returns: Mapping[str, float] | None = None,
        resample: int | None = None,
        current_weights: Mapping[str, float] | None = None,
        max_turnover: float | None = None,
        min_trade: float | None = None,
        max_trades: int | None = None,
        benchmark_weights: Mapping[str, float] | None = None,
        max_tracking_error: float | None = None,
        cash: BoundLike | None = None,
        frontier: bool = False,
        name: str | None = None,
    ) -> _types.Optimisation:
        """The optimisation of the risky assets by `method`, fully invested: the weights in
        ``result.results.weights``, and the optimisation report in ``result.report``, which
        ``print(result)`` prints and ``result.show()`` shows. `name` titles the report. The
        methods:

        - ``"maximum_sharpe"`` (the default), ``"minimum_variance"``,
          ``"maximum_diversification"`` and ``"hierarchical_risk_parity"`` (with `linkage`);
        - ``"mean_variance"`` and ``"robust_mean_variance"``, with `risk_aversion` (and
          `uncertainty`, an :class:`~finalytics.types.Ellipsoid` or a
          :class:`~finalytics.types.Box`);
        - ``"target_return"`` and ``"target_volatility"``, with an annual `target`;
        - ``"minimum_cvar"``, ``"minimum_semivariance"`` and ``"minimum_cdar"``, measured on
          the periods' returns: the expected shortfall at `confidence` (0.95 by default), the
          mean square of the returns below zero, and the conditional drawdown at risk at
          `confidence` of the returns added up period by period; with `target`, each keeps an
          annual expected return of at least `target`;
        - ``"risk_parity"``, every asset with the same share of the risk, and
          ``"risk_budgeting"``, with shares given by `budgets` by asset;
        - ``"equal_weight"`` and ``"inverse_volatility"``, the baselines estimated portfolios
          must beat.

        `max_weight` is the largest weight of any asset, such as 0.4; `bounds` limits every
        weight, ``(0, 1)`` by default; `asset_bounds` limits particular assets, such as
        ``{"Gold": (0, 0.1)}``; `groups` limits the total weight of groups of assets, such as
        ``{"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)}``, or the assets of a class,
        ``{"Equity": ("asset_class", 0.3, 0.6)}`` (see `classes`); `min_effective_assets`, such
        as 5, spreads the weights over at least as many assets as 5 equal weights would be,
        holding ``1 / sum(w ** 2)`` at or above it. Equal weight, inverse volatility, risk
        parity, risk budgeting and hierarchical risk parity are fully invested and long only,
        and take no limits. Expected returns are the assets' estimates (see `mean_returns`)
        unless `expected_returns` gives others.

        ``resample=200`` averages the optimal weights for 200 histories simulated from the
        estimates, each as long as the assets' returns (Michaud's resampling), which makes the
        weights less sensitive to errors in the estimates; ``result.results.resampled`` says how
        much each weight varied. It applies to the methods that use estimates, other than those
        measured on the periods' returns.

        `current_weights` are the weights held now, by asset, adding up to one with any cash
        on the risk-free asset, such as ``{"US Equity": 0.5, "Bonds": 0.45, "T-Bills": 0.05}``:
        the report lists the trades from them, and they are where the trade limits start.
        `max_turnover` limits the one-way turnover from them, ``sum(abs(w - current)) / 2``
        with the cash, such as 0.2; `min_trade` makes each trade at least that weight or none,
        and `max_trades` limits how many assets are traded (both met by leaving trades out one
        by one, so not for the methods measured on the periods' returns).
        `benchmark_weights` gives the report the tracking error against a benchmark, and
        `max_tracking_error` limits it, a year, such as 0.03.

        ``cash=(0, 0.2)`` lets the weights hold between none and 20% in cash, riskless: in the
        risk-free asset, earning its return, or, without one, in ``"Cash"`` at the risk-free
        rate. The frontier then includes the cash. Without it, the weights are fully invested
        in the risky assets. The Sharpe and diversification methods take no limits on cash.

        ``frontier=True`` draws the efficient frontier under the same limits in the report. It
        takes 30 more optimisations, so it adds time that grows with the number of assets, to
        seconds for hundreds of assets; without it, the report shows the weights against each
        asset and equal weight.
        """
        options = self._options(
            method,
            risk_free_rate=risk_free_rate,
            target=target,
            risk_aversion=risk_aversion,
            uncertainty=uncertainty,
            confidence=confidence,
            budgets=budgets,
            linkage=linkage,
            bounds=bounds,
            max_weight=max_weight,
            asset_bounds=asset_bounds,
            groups=groups,
            min_effective_assets=min_effective_assets,
            expected_returns=expected_returns,
            resample=resample,
            current_weights=current_weights,
            max_turnover=max_turnover,
            min_trade=min_trade,
            max_trades=max_trades,
            benchmark_weights=benchmark_weights,
            max_tracking_error=max_tracking_error,
            cash=cash,
        )
        options = dataclasses.replace(options, frontier=bool(frontier))
        result = loads(_types.Optimisation, call(self._native.optimise, dumps(options, "options")))
        results = result.results
        if results.weights is not None:
            # In the order of the assets, rather than of their names, with any cash at the
            # risk-free rate last.
            order = {label: i for i, label in enumerate(self.labels)}
            weights = dict(sorted(results.weights.items(), key=lambda item: order.get(item[0], len(order))))
            result = dataclasses.replace(result, results=dataclasses.replace(results, weights=weights))
        if name is not None:
            result = dataclasses.replace(result, report=dataclasses.replace(result.report, name=name))
        return result

    def frontier(
        self,
        *,
        points: int = 20,
        bounds: BoundLike | None = None,
        max_weight: float | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        min_effective_assets: float | None = None,
        expected_returns: Mapping[str, float] | None = None,
        current_weights: Mapping[str, float] | None = None,
        max_turnover: float | None = None,
        min_trade: float | None = None,
        max_trades: int | None = None,
        benchmark_weights: Mapping[str, float] | None = None,
        max_tracking_error: float | None = None,
        cash: BoundLike | None = None,
    ) -> list[_types.FrontierPortfolio]:
        """The efficient frontier: `points` portfolios of the risky assets, from the one with
        the least variance to the one with the largest expected return, under the limits given
        as for :meth:`optimise`, trade and tracking-error limits and cash included."""
        options = self._options(
            "maximum_sharpe",
            bounds=bounds,
            max_weight=max_weight,
            asset_bounds=asset_bounds,
            groups=groups,
            min_effective_assets=min_effective_assets,
            expected_returns=expected_returns,
            current_weights=current_weights,
            max_turnover=max_turnover,
            min_trade=min_trade,
            max_trades=max_trades,
            benchmark_weights=benchmark_weights,
            max_tracking_error=max_tracking_error,
            cash=cash,
        )
        return loads(list[_types.FrontierPortfolio], call(self._native.frontier, int(points), dumps(options, "options")))

    def compare(
        self,
        *,
        methods: Sequence[_types.Method | tuple[str, Mapping[str, float]] | _types.FixedWeights] = (),
        weights: Mapping[str, Mapping[str, float]] | None = None,
        name: str | None = None,
        validation: _types.Validation | str = "in_sample",
        rebalance: _types.RebalanceSchedule | str = "quarterly",
        trading_cost_bps: float = 0.0,
        benchmark: Any = None,
        benchmark_name: str | None = None,
        max_weight: float | None = None,
        bounds: BoundLike | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        min_effective_assets: float | None = None,
        target: float | None = None,
        risk_aversion: float | None = None,
        uncertainty: _types.ReturnUncertainty | None = None,
        confidence: float | None = None,
        budgets: Mapping[str, float] | None = None,
        linkage: _types.Linkage | None = None,
        expected_returns: Mapping[str, float] | None = None,
        risk_free_rate: float | None = None,
        resample: int | None = None,
        max_turnover: float | None = None,
        min_trade: float | None = None,
        max_trades: int | None = None,
        benchmark_weights: Mapping[str, float] | None = None,
        max_tracking_error: float | None = None,
        cash: BoundLike | None = None,
        start_value: float | None = None,
        flows: FlowsLike = (),
    ) -> _types.Comparison:
        """The comparison of `methods`, each backtested on these assets with the same limits,
        rebalancing, validation and trading costs and scored on the periods it was tested on:
        the comparison report in ``result.report``, and every return and trade in
        ``result.results``. ``print(result)`` prints the report and ``result.show()`` shows it,
        inline in a notebook or in a browser. `name` titles the report. The results and the
        report keep the strategies in the order given.

        `weights` adds fixed weights by name, such as ``{"60/40": {"US Equity": 0.6, "Bonds":
        0.4}}``, after the methods, held and rebalanced as the methods are and scored on the same
        periods; they are not fitted to any returns, so they have no run in sample. Fixed
        weights may also come among the methods, as a name and weights: ``methods=[("60/40",
        {...}), "minimum_variance"]``.

        - `validation`: ``"in_sample"`` (the default); :class:`~finalytics.Holdout`, such as
          ``fl.Holdout(start="2016-11-01")``, which fits on the history before the date and
          tests on the rest; or :class:`~finalytics.WalkForward`, such as
          ``fl.WalkForward(years=5)``, which refits at each scheduled rebalance on the last
          five years (or all the history so far, with ``expanding=True``).
        - `rebalance`: ``"monthly"``, ``"quarterly"`` (the default), ``"yearly"``,
          ``"never"``, :class:`~finalytics.Drift`, such as ``fl.Drift(band=0.05)``, or
          :class:`~finalytics.Every`, such as ``fl.Every(periods=21)`` for about once a month
          with daily returns.
        - `trading_cost_bps`: the cost of trading in basis points of the value traded.
        - `benchmark`: weights by asset, such as ``{"US Equity": 0.6, "Bonds": 0.4}``, or an
          index's prices, as a pandas DataFrame indexed by date with one column;
          `benchmark_name` names it (``"Benchmark"`` by default, or the prices' column name).
        - `start_value` and `flows`: money to follow as well as returns. Each run starts with
          `start_value`, and `flows` pays in (positive) or takes out (negative) amounts on
          dates, such as ``{"2020-01-31": 500}``, at the end of the period each falls in; the
          reports then give each run's ending value and money-weighted return.

        The limits and parameters are those of :meth:`optimise`; they apply to every method
        that takes them. `max_turnover`, `min_trade` and `max_trades` limit the trades at each
        refit of a walk-forward validation, from the weights held then (the first fit enters
        from cash); `max_tracking_error` applies at every fit.
        """
        limits = dict(
            max_weight=max_weight,
            bounds=bounds,
            asset_bounds=asset_bounds,
            groups=groups,
            min_effective_assets=min_effective_assets,
            target=target,
            risk_aversion=risk_aversion,
            uncertainty=uncertainty,
            confidence=confidence,
            budgets=budgets,
            linkage=linkage,
            expected_returns=expected_returns,
            risk_free_rate=risk_free_rate,
            resample=resample,
            max_turnover=max_turnover,
            min_trade=min_trade,
            max_trades=max_trades,
            benchmark_weights=benchmark_weights,
            max_tracking_error=max_tracking_error,
            cash=cash,
        )
        if isinstance(methods, str):
            raise invalid(("methods", f"must be a list of methods, such as [{methods!r}]"))
        strategies: list[Any] = []
        for k, item in enumerate(methods):
            if isinstance(item, _types.FixedWeights):
                strategies.append(item)
            elif isinstance(item, tuple) and len(item) == 2 and isinstance(item[0], str):
                label, held = item
                if not isinstance(held, Mapping):
                    raise invalid((f"methods[{k}]", "fixed weights are a name and weights by asset"))
                strategies.append(_types.FixedWeights(name=label, weights={a: float(v) for a, v in held.items()}))
            else:
                strategies.append(_types.OptimisedStrategy(options=self._options(item, **limits)))
        strategies += [
            _types.FixedWeights(name=label, weights={k: float(v) for k, v in held.items()})
            for label, held in (weights or {}).items()
        ]
        if not strategies:
            raise invalid(("methods", "give the methods to compare, or fixed weights"))
        if not any(isinstance(s, _types.OptimisedStrategy) for s in strategies):
            given = [key for key, value in limits.items() if value is not None]
            if given:
                raise invalid(*((f"options.{key}", "fixed weights take no limits or parameters") for key in given))
        options = _backtest_options(
            strategies, validation, rebalance, trading_cost_bps, benchmark, benchmark_name, start_value, flows
        )
        result = loads(_types.Comparison, call(self._native.compare, dumps(options, "options")))
        if name is not None:
            result = dataclasses.replace(result, report=dataclasses.replace(result.report, name=name))
        return result

    def backtest(
        self,
        *,
        method: _types.Method | None = None,
        weights: Mapping[str, float] | None = None,
        name: str | None = None,
        validation: _types.Validation | str = "in_sample",
        rebalance: _types.RebalanceSchedule | str = "quarterly",
        trading_cost_bps: float = 0.0,
        benchmark: Any = None,
        benchmark_name: str | None = None,
        max_weight: float | None = None,
        bounds: BoundLike | None = None,
        asset_bounds: Mapping[str, BoundLike] | None = None,
        groups: Mapping[str, GroupLike] | None = None,
        min_effective_assets: float | None = None,
        target: float | None = None,
        risk_aversion: float | None = None,
        uncertainty: _types.ReturnUncertainty | None = None,
        confidence: float | None = None,
        budgets: Mapping[str, float] | None = None,
        linkage: _types.Linkage | None = None,
        expected_returns: Mapping[str, float] | None = None,
        risk_free_rate: float | None = None,
        resample: int | None = None,
        max_turnover: float | None = None,
        min_trade: float | None = None,
        max_trades: int | None = None,
        benchmark_weights: Mapping[str, float] | None = None,
        max_tracking_error: float | None = None,
        cash: BoundLike | None = None,
        start_value: float | None = None,
        flows: FlowsLike = (),
    ) -> _types.Backtest:
        """The backtest of one strategy: a `method`, fitted as `validation` says, or fixed
        `weights` by asset, such as ``{"US Equity": 0.6, "Bonds": 0.4}`` (a weight on the
        risk-free asset is held as cash), against the `benchmark` if one is given: the backtest
        report in ``result.report``, and every return and trade in ``result.results``. `name`
        names the strategy and titles the report. The other keywords are as for
        :meth:`compare`; fixed weights take no limits or parameters."""
        if method is None and weights is None:
            raise invalid(("method", "give a method, or fixed weights"))
        if method is not None and weights is not None:
            raise invalid(("weights", "give a method or fixed weights, not both"))
        limits = dict(
            max_weight=max_weight,
            bounds=bounds,
            asset_bounds=asset_bounds,
            groups=groups,
            min_effective_assets=min_effective_assets,
            target=target,
            risk_aversion=risk_aversion,
            uncertainty=uncertainty,
            confidence=confidence,
            budgets=budgets,
            linkage=linkage,
            expected_returns=expected_returns,
            risk_free_rate=risk_free_rate,
            resample=resample,
            max_turnover=max_turnover,
            min_trade=min_trade,
            max_trades=max_trades,
            benchmark_weights=benchmark_weights,
            max_tracking_error=max_tracking_error,
            cash=cash,
        )
        strategy: Any
        if weights is not None:
            given = [key for key, value in limits.items() if value is not None]
            if given:
                raise invalid(*((f"options.{key}", "fixed weights take no limits or parameters") for key in given))
            strategy = _types.FixedWeights(name=name, weights={k: float(v) for k, v in weights.items()})
        else:
            assert method is not None
            strategy = _types.OptimisedStrategy(name=name, options=self._options(method, **limits))
        options = _backtest_options(
            [strategy], validation, rebalance, trading_cost_bps, benchmark, benchmark_name, start_value, flows
        )
        return loads(_types.Backtest, call(self._native.backtest, dumps(options, "options")))

    def plan(
        self,
        goal: _types.Goal | None = None,
        *,
        weights: MixLike,
        inflation: float | Any,
        flows: Sequence[_types.PlanFlow] = (),
        start_value: float = 0.0,
        alternatives: Mapping[str, MixLike] | Sequence[MixLike] = (),
        fee: float = 0.0,
        chance: float | tuple[float, float] = 0.8,
        model: _types.Model | str = "bootstrap",
        expected_returns: Mapping[str, float] | None = None,
        start: str | datetime.date | None = None,
        end: str | datetime.date | None = None,
        simulations: int = 10_000,
        seed: int = 0,
        currency: str | None = None,
        name: str | None = None,
    ) -> _types.Plan:
        """The chance that a plan of saving, spending and investing meets one `goal`, from
        `simulations` markets drawn from these assets' monthly returns: the planning report in
        ``plan.report``, and the raw results in ``plan.results``. ``print(plan)`` prints the
        report and ``plan.show()`` shows it, inline in a notebook or in a browser.

        - `goal`: :class:`~finalytics.Target`, an amount by a month, such as
          ``fl.Target(amount=150_000, by="2030-06")``; :class:`~finalytics.Payments`, an
          amount paid regularly, such as ``fl.Payments(amount=4_000, start="2046-11",
          end="2076-10")``; or none, for a projection to the month `end`.
        - `weights`: what the money is invested in, rebalanced every month: weights by asset,
          such as ``{"US Equity": 0.6, "Bonds": 0.4}``; a name and weights, such as
          ``("Balanced 60/40", {...})``; a :class:`~finalytics.GlidePath`; or a result with
          weights, such as a frontier portfolio.
        - `inflation`: the rate a year, such as ``0.025``, or a price index of one column, such
          as a consumer price index, as a pandas DataFrame indexed by date. Every amount is in
          today's money: returns are after inflation and fees.
        - `flows`: money paid in and taken out, such as ``fl.Saving(amount=1_500,
          every="month")``, which runs until the goal, or ``fl.Spending(name="Car",
          amount=25_000, on="2028-03")``.
        - `alternatives`: other mixes to compare on the same simulations, by name.
        - `chance`: the chance wanted (0.8 by default), or the chance wanted and the chance from
          which the plan has room to spare, such as ``(0.85, 0.95)``.
        - `model`: ``"bootstrap"`` (blocks of past months, the default), ``"normal"`` or
          ``"historical"``, or :class:`~finalytics.Bootstrap` with its `block` length.
        - `start`: the first month, the month after the assets' last by default.
        """
        wanted, spare = (chance if isinstance(chance, tuple) else (chance, None))
        if isinstance(model, str):
            model = _named("model", model, _MODELS)
        if isinstance(inflation, (int, float)) and not isinstance(inflation, bool):
            inflation = _types.InflationRate(rate=float(inflation))
        elif inflation is not None and not isinstance(inflation, (_types.InflationRate, _types.InflationIndex)):
            inflation = _types.InflationIndex(prices=_price_table(inflation))
        if isinstance(alternatives, Mapping):
            mixes = [_mix((key, value)) for key, value in alternatives.items()]
        else:
            mixes = [_mix(value) for value in alternatives]
        if name is not None and goal is not None and goal.name is None:
            goal = dataclasses.replace(goal, name=name)
        options = _types.PlanOptions(
            goal=goal,
            chance=float(wanted),
            spare=None if spare is None else float(spare),
            start=_month(start),
            end=_month(end),
            start_value=float(start_value),
            flows=list(flows),
            weights=_mix(weights),
            alternatives=mixes,
            fee=float(fee),
            inflation=inflation,
            model=model,
            expected_returns=None if expected_returns is None else {k: float(v) for k, v in expected_returns.items()},
            simulations=int(simulations),
            seed=int(seed),
            currency=currency,
            name=name,
        )
        return loads(_types.Plan, call(self._native.plan, dumps(options, "options")))

    def __repr__(self) -> str:
        return f"Assets({len(self.labels)} assets, {len(self.dates)} periods, {self.periods_per_year:g} a year)"


def _backtest_options(
    strategies: list[Any],
    validation: _types.Validation | str,
    rebalance: _types.RebalanceSchedule | str,
    trading_cost_bps: float,
    benchmark: Any,
    benchmark_name: str | None = None,
    start_value: float | None = None,
    flows: FlowsLike = (),
) -> _types.BacktestOptions:
    if isinstance(validation, str):
        validation = _named("validation", validation, {"in_sample": _types.InSample()})
    if isinstance(rebalance, str):
        rebalance = _named("rebalance", rebalance, _SCHEDULES)
    if isinstance(benchmark, Mapping):
        benchmark = _types.BenchmarkWeights(weights={k: float(v) for k, v in benchmark.items()})
    elif benchmark is not None and not isinstance(
        benchmark, (_types.BenchmarkWeights, _types.BenchmarkPrices)
    ):
        benchmark = _types.BenchmarkPrices(prices=_price_table(benchmark))
    if benchmark_name is not None:
        if benchmark is None:
            raise invalid(("benchmark_name", "names a benchmark: give the benchmark too"))
        benchmark = dataclasses.replace(benchmark, name=benchmark_name)
    items = flows.items() if isinstance(flows, Mapping) else flows
    return _types.BacktestOptions(
        strategies=strategies,
        validation=validation,
        rebalance=rebalance,
        trading_cost_bps=float(trading_cost_bps),
        benchmark=benchmark,
        start_value=None if start_value is None else float(start_value),
        flows=[
            _types.CashFlow(
                date=datetime.date.fromisoformat(_arrays.date(date)),
                amount=float(amount),
            )
            for date, amount in items
        ],
    )


_MODELS: dict[str, Any] = {
    "bootstrap": _types.Bootstrap(),
    "normal": _types.Normal(),
    "historical": _types.Historical(),
}


def _mix(value: Any) -> Any:
    """An investment mix from weights by asset, a (name, weights) pair, a mix, or anything with
    weights, such as a frontier portfolio or an optimisation's result."""
    if isinstance(value, (_types.FixedMix, _types.GlidePath)):
        return value
    name = None
    if isinstance(value, tuple) and len(value) == 2 and isinstance(value[0], str):
        name, value = value
        if isinstance(value, (_types.FixedMix, _types.GlidePath)):
            return dataclasses.replace(value, name=name)
    if not isinstance(value, Mapping):
        found = getattr(value, "weights", None)
        if found is None and hasattr(value, "results"):
            found = getattr(value.results, "weights", None)
        if not isinstance(found, Mapping):
            raise TypeError(
                "a mix must be weights by asset, a (name, weights) pair, a FixedMix or GlidePath, "
                "or a result with weights"
            )
        value = found
    return _types.FixedMix(name=name, weights={str(k): float(v) for k, v in value.items()})


def _month(value: str | datetime.date | None) -> str | None:
    """A month as text, from text or a date (a datetime or a pandas Timestamp included)."""
    if value is None or isinstance(value, str):
        return value
    return _arrays.date(value)


_SCHEDULES: dict[str, Any] = {
    "monthly": _types.Monthly(),
    "quarterly": _types.Quarterly(),
    "yearly": _types.Yearly(),
    "never": _types.Never(),
}


def _named(option: str, name: str, choices: Mapping[str, Any]) -> Any:
    """The choice called `name` for `option`."""
    if name not in choices:
        known = ", ".join(repr(choice) for choice in choices)
        raise invalid((option, f"{name!r} is not one of {known}"))
    return choices[name]
