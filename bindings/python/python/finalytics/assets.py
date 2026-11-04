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

__all__ = ["Assets"]

BoundLike = _types.Bound | tuple[float, float]
"""Limits on a weight: a :class:`~finalytics.types.Bound` or a ``(min, max)`` pair."""

GroupLike = _types.AssetGroup | tuple[Sequence[str], float, float]
"""A group of assets and the limits on their total weight: a
:class:`~finalytics.types.AssetGroup` or an ``(assets, min, max)`` triple."""


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

    ::

        import finalytics as fl

        assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
        result = assets.optimise(method="maximum_sharpe", max_weight=0.4)
        result.weights
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
    ):
        if isinstance(prices, (str, os.PathLike)):
            from finalytics.portfolio import _text

            native = call(_finalytics.NativeAssets.from_csv, _text(prices))
        else:
            native = call(_finalytics.NativeAssets.from_prices, dumps(_price_table(prices)))
        self._native = self._configure(
            native, risk_free_rate, risk_free_asset, periods_per_year, covariance, mean_returns
        )

    @staticmethod
    def _configure(
        native: Any,
        risk_free_rate: float | None,
        risk_free_asset: str | None,
        periods_per_year: float | None,
        covariance: str | _types.CovarianceMethod,
        mean_returns: str | _types.MeanMethod,
    ) -> Any:
        settings = {
            "risk_free_rate": None if risk_free_rate is None else float(risk_free_rate),
            "risk_free_asset": risk_free_asset,
            "periods_per_year": None if periods_per_year is None else float(periods_per_year),
            "covariance": json.loads(_covariance(covariance)),
            "mean_returns": json.loads(_mean(mean_returns)),
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
            native, risk_free_rate, risk_free_asset, periods_per_year, covariance, mean_returns
        )
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
        }
        numbers.update({f"budgets.{k}": v for k, v in (budgets or {}).items()})
        numbers.update({f"expected_returns.{k}": v for k, v in (expected_returns or {}).items()})
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
        ``{"Equity": (["US Equity", "Intl Equity"], 0.3, 0.6)}``; `min_effective_assets`, such
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
        )
        options = dataclasses.replace(options, frontier=bool(frontier))
        result = loads(_types.Optimisation, call(self._native.optimise, dumps(options, "options")))
        results = result.results
        if results.weights is not None:
            # In the order of the assets, rather than of their names.
            order = {label: i for i, label in enumerate(self.labels)}
            weights = dict(sorted(results.weights.items(), key=lambda item: order[item[0]]))
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
    ) -> list[_types.FrontierPortfolio]:
        """The efficient frontier: `points` portfolios of the risky assets, from the one with
        the least variance to the one with the largest expected return, under the limits given
        as for :meth:`optimise`."""
        options = self._options(
            "maximum_sharpe",
            bounds=bounds,
            max_weight=max_weight,
            asset_bounds=asset_bounds,
            groups=groups,
            min_effective_assets=min_effective_assets,
            expected_returns=expected_returns,
        )
        return loads(list[_types.FrontierPortfolio], call(self._native.frontier, int(points), dumps(options, "options")))

    def compare(
        self,
        *,
        methods: Sequence[_types.Method] = (),
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
    ) -> _types.Comparison:
        """The comparison of `methods`, each backtested on these assets with the same limits,
        rebalancing, validation and trading costs and scored on the periods it was tested on:
        the comparison report in ``result.report``, and every return and trade in
        ``result.results``. ``print(result)`` prints the report and ``result.show()`` shows it,
        inline in a notebook or in a browser. `name` titles the report.

        `weights` adds fixed weights by name, such as ``{"60/40": {"US Equity": 0.6, "Bonds":
        0.4}}``, held and rebalanced as the methods are and scored on the same periods; they are
        not fitted to any returns, so they have no run in sample.

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
          index's prices, such as ``fl.datasets.sp500_index()``; `benchmark_name` names it
          (``"Benchmark"`` by default, or the prices' column name).

        The limits and parameters are those of :meth:`optimise`; they apply to every method
        that takes them.
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
        )
        if isinstance(methods, str):
            raise invalid(("methods", f"must be a list of methods, such as [{methods!r}]"))
        strategies: list[Any] = [
            _types.OptimisedStrategy(options=self._options(method, **limits)) for method in methods
        ]
        strategies += [
            _types.FixedWeights(name=label, weights={k: float(v) for k, v in held.items()})
            for label, held in (weights or {}).items()
        ]
        if not strategies:
            raise invalid(("methods", "give the methods to compare, or fixed weights"))
        options = _backtest_options(strategies, validation, rebalance, trading_cost_bps, benchmark, benchmark_name)
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
        options = _backtest_options([strategy], validation, rebalance, trading_cost_bps, benchmark, benchmark_name)
        return loads(_types.Backtest, call(self._native.backtest, dumps(options, "options")))

    def __repr__(self) -> str:
        return f"Assets({len(self.labels)} assets, {len(self.dates)} periods, {self.periods_per_year:g} a year)"


def _backtest_options(
    strategies: list[Any],
    validation: _types.Validation | str,
    rebalance: _types.RebalanceSchedule | str,
    trading_cost_bps: float,
    benchmark: Any,
    benchmark_name: str | None = None,
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
    return _types.BacktestOptions(
        strategies=strategies,
        validation=validation,
        rebalance=rebalance,
        trading_cost_bps=float(trading_cost_bps),
        benchmark=benchmark,
    )


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
