"""Portfolio optimisation: objectives, constraints, the efficient frontier and rebalancing.

::

    import finalytics as fl

    problem = fl.opt.Problem.from_covariance(cov, expected_returns=mu)
    solution = problem.solve(fl.opt.MaximumSharpe(risk_free_rate=0.03))

Black–Litterman expected returns blend the returns implied by market weights with views::

    estimate = fl.opt.black_litterman(cov, [fl.opt.View(portfolio={"bonds": 1.0}, expected_return=0.05)],
                                      market_weights=weights, risk_aversion=2.5)
    solution = fl.opt.Problem.from_black_litterman(estimate).solve(fl.opt.MeanVariance(risk_aversion=2.5))
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

from finalytics import _arrays, _finalytics, _types
from finalytics._convert import dumps, loads
from finalytics._errors import call
from finalytics._types import (
    AssetBound,
    BlackLitterman,
    BlackLittermanEstimate,
    Bound,
    Box,
    CombinedRebalance,
    Confidence,
    Constraints,
    Ellipsoid,
    FactorExposureLimit,
    FactorModel,
    FactorModelEstimate,
    Fees,
    FrontierPoint,
    GroupLimit,
    HierarchicalRiskParity,
    Linkage,
    MaximumDiversification,
    MarketPrior,
    MaximumSharpe,
    MeanVariance,
    Method,
    MinimumCvar,
    MinimumVariance,
    Objective,
    Prior,
    Proportional,
    ProposedConversion,
    ProposedTrade,
    Rebalance,
    RebalanceOptions,
    ResampledSolution,
    Resampling,
    ReturnsPrior,
    ReturnUncertainty,
    RiskBudgeting,
    RobustMeanVariance,
    Side,
    Solution,
    SolveStatus,
    TargetReturn,
    TargetVolatility,
    TaxRates,
    TrackingErrorLimit,
    TradeLimits,
    Transaction,
    Turnover,
    Variance,
    View,
    ViewUncertainty,
)
from finalytics.risk import _matrix

__all__ = [
    "AssetBound",
    "BlackLitterman",
    "BlackLittermanEstimate",
    "Bound",
    "Box",
    "CombinedRebalance",
    "Confidence",
    "Constraints",
    "Ellipsoid",
    "FactorExposureLimit",
    "Fees",
    "FrontierPoint",
    "GroupLimit",
    "HierarchicalRiskParity",
    "Linkage",
    "MaximumDiversification",
    "MarketPrior",
    "MaximumSharpe",
    "MeanVariance",
    "Method",
    "MinimumCvar",
    "MinimumVariance",
    "Objective",
    "Prior",
    "Problem",
    "Proportional",
    "ProposedConversion",
    "ProposedTrade",
    "Rebalance",
    "RebalanceOptions",
    "ResampledSolution",
    "Resampling",
    "ReturnUncertainty",
    "ReturnsPrior",
    "RiskBudgeting",
    "RobustMeanVariance",
    "Side",
    "Solution",
    "SolveStatus",
    "TargetReturn",
    "TargetVolatility",
    "TaxRates",
    "TrackingErrorLimit",
    "TradeLimits",
    "Turnover",
    "Variance",
    "View",
    "ViewUncertainty",
    "black_litterman",
    "implied_risk_aversion",
    "transactions",
]


class Problem(_types.Problem):
    """A portfolio optimisation problem: the assets, estimates of their risk and return, and the
    constraints. Risk comes from a covariance matrix or a factor model; the risk estimates and
    expected returns should be in the same units, for example both annualised; scenario returns,
    for minimum CVaR, are per period."""

    @classmethod
    def from_covariance(
        cls,
        covariance: Any,
        *,
        labels: Sequence[str] | None = None,
        expected_returns: Any = None,
        constraints: Constraints | None = None,
        scenarios: Any = None,
    ) -> Problem:
        """A problem over the assets of a covariance matrix: a
        :class:`~finalytics.types.Matrix`, a covariance estimate, a square array or a pandas
        DataFrame."""
        matrix = _matrix(covariance) if labels is None else _matrix(_labelled(covariance, labels))
        return cls(
            labels=list(matrix.labels),
            covariance=matrix,
            expected_returns=None if expected_returns is None else np.asarray(expected_returns, float).tolist(),
            scenarios=None if scenarios is None else _scenarios(scenarios, matrix.labels),
            constraints=constraints or Constraints(),
        )

    @classmethod
    def from_factor_model(
        cls,
        model: FactorModel | FactorModelEstimate,
        *,
        expected_returns: Any = None,
        constraints: Constraints | None = None,
        scenarios: Any = None,
    ) -> Problem:
        """A problem over the assets of a factor model, which the solver uses in factor form, so
        that large problems solve much faster than with a full covariance matrix. Use
        :class:`FactorExposureLimit` constraints to limit the portfolio's factor exposures."""
        model = model.model if isinstance(model, FactorModelEstimate) else model
        return cls(
            labels=list(model.assets),
            factor_model=model,
            expected_returns=None if expected_returns is None else np.asarray(expected_returns, float).tolist(),
            scenarios=None if scenarios is None else _scenarios(scenarios, model.assets),
            constraints=constraints or Constraints(),
        )

    @classmethod
    def from_scenarios(
        cls, scenarios: Any, *, labels: Sequence[str] | None = None, constraints: Constraints | None = None
    ) -> Problem:
        """A problem over the assets of scenario returns (one row per scenario, one column per
        asset), for minimum CVaR."""
        if labels is None and hasattr(scenarios, "columns"):
            labels = [str(c) for c in scenarios.columns]
        array = np.asarray(scenarios, dtype=np.float64)
        labels = list(labels) if labels is not None else [f"asset {i}" for i in range(array.shape[1])]
        return cls(labels=labels, scenarios=_scenarios(array, labels), constraints=constraints or Constraints())

    @classmethod
    def from_black_litterman(cls, estimate: BlackLittermanEstimate, *, constraints: Constraints | None = None) -> Problem:
        """A problem with the posterior covariance and expected returns of a Black–Litterman
        estimate."""
        return cls(
            labels=list(estimate.labels),
            covariance=estimate.posterior_covariance,
            expected_returns=list(estimate.posterior_returns),
            constraints=constraints or Constraints(),
        )

    def solve(self, objective: Objective) -> Solution:
        """The optimal portfolio for `objective`. A problem with no solution, such as one whose
        constraints cannot all hold, returns a solution whose ``status`` says why and whose
        ``weights`` are None."""
        return loads(Solution, call(_finalytics.solve, dumps(self), dumps(objective)))

    def resampled(
        self, objective: Objective, *, observations: int, samples: int = 100, seed: int = 0
    ) -> ResampledSolution:
        """Resampled optimisation: the average of the optimal portfolios for `samples` sets of
        estimates, each re-made from a simulated history of `observations` periods (the length of
        the history the estimates came from). The same `seed` gives the same result."""
        resampling = Resampling(samples=int(samples), observations=int(observations), seed=int(seed))
        text = call(_finalytics.resampled, dumps(self), dumps(objective), dumps(resampling))
        return loads(ResampledSolution, text)

    def frontier(self, points: int = 20) -> list[FrontierPoint]:
        """The efficient frontier: `points` portfolios from the minimum-variance portfolio to the
        largest attainable expected return."""
        return loads(list[FrontierPoint], call(_finalytics.frontier, dumps(self), int(points)))


def _labelled(values: Any, labels: Sequence[str]) -> Any:
    from finalytics.risk import matrix

    return matrix(values, labels)


def _scenarios(values: Any, labels: Sequence[str]) -> _types.ReturnsTable:
    array = np.asarray(values, dtype=np.float64)
    if array.ndim != 2 or array.shape[1] != len(labels):
        raise ValueError("scenarios need one row per scenario and one column per asset")
    return _types.ReturnsTable(labels=list(labels), columns=array.T.tolist())


def _aligned(values: Any, labels: Sequence[str], name: str) -> list[float]:
    """Values given in the order of `labels`, or as a mapping (a dict or a pandas Series) from
    label to value."""
    if hasattr(values, "items"):
        missing = [label for label in labels if label not in values]
        if missing:
            raise ValueError(f"{name} has no value for {', '.join(missing)}")
        return [float(values[label]) for label in labels]
    return np.asarray(values, dtype=np.float64).tolist()


def black_litterman(
    covariance: Any,
    views: Sequence[View] = (),
    *,
    market_weights: Any = None,
    risk_aversion: float = 2.5,
    risk_free_rate: float = 0.0,
    prior_returns: Any = None,
    prior: Prior | None = None,
    tau: float = 0.05,
    labels: Sequence[str] | None = None,
) -> BlackLittermanEstimate:
    """The Black–Litterman posterior expected returns and covariance.

    The prior is the equilibrium implied by `market_weights` (market capitalisations or weights,
    in the order of the covariance matrix or as a mapping from label), `risk_aversion` and
    `risk_free_rate`; or `prior_returns`; or a :class:`MarketPrior` or :class:`ReturnsPrior`.
    Each :class:`View` gives the expected return of a portfolio of assets, with an uncertainty
    that is proportional to its variance by default."""
    matrix = _matrix(covariance) if labels is None else _matrix(_labelled(covariance, labels))
    if prior is None:
        if market_weights is not None:
            prior = MarketPrior(
                weights=_aligned(market_weights, matrix.labels, "market_weights"),
                risk_aversion=float(risk_aversion),
                risk_free_rate=float(risk_free_rate),
            )
        elif prior_returns is not None:
            prior = ReturnsPrior(returns=_aligned(prior_returns, matrix.labels, "prior_returns"))
        else:
            raise ValueError("give market_weights, prior_returns or a prior")
    model = BlackLitterman(covariance=matrix, prior=prior, views=list(views), tau=float(tau))
    return loads(BlackLittermanEstimate, call(_finalytics.black_litterman, dumps(model)))


def implied_risk_aversion(returns: Any, periods_per_year: float = 252, risk_free_rate: float = 0.0) -> float:
    """The market's risk aversion implied by its returns: the annualised excess return over the
    annualised variance."""
    buffer = _arrays.floats_to_buffer(np.asarray(returns, dtype=np.float64))
    return call(_finalytics.implied_risk_aversion, buffer, float(periods_per_year), float(risk_free_rate))


def transactions(plan: Rebalance, prefix: str = "rebalance-") -> list[Transaction]:
    """A rebalancing proposal as transactions on its date (sales naming their lots, then
    conversions, then purchases), ready to add to the portfolio."""
    return loads(list[Transaction], call(_finalytics.rebalance_transactions, dumps(plan), prefix))
