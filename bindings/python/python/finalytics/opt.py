"""Portfolio optimisation: objectives, constraints, the efficient frontier and rebalancing.

::

    import finalytics as fl

    problem = fl.opt.Problem.from_covariance(cov, expected_returns=mu)
    solution = problem.solve(fl.opt.MaximumSharpe(risk_free_rate=0.03))
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

from finalytics import _finalytics, _types
from finalytics._convert import dumps, loads
from finalytics._errors import call
from finalytics._types import (
    AssetBound,
    Bound,
    Constraints,
    FrontierPoint,
    GroupLimit,
    HierarchicalRiskParity,
    Linkage,
    MaximumDiversification,
    MaximumSharpe,
    MeanVariance,
    Method,
    MinimumCvar,
    MinimumVariance,
    Objective,
    ProposedConversion,
    ProposedTrade,
    Rebalance,
    RebalanceOptions,
    RiskBudgeting,
    Side,
    Solution,
    SolveStatus,
    TargetReturn,
    TargetVolatility,
    TrackingErrorLimit,
    Transaction,
    Turnover,
)
from finalytics.risk import _matrix

__all__ = [
    "AssetBound",
    "Bound",
    "Constraints",
    "FrontierPoint",
    "GroupLimit",
    "HierarchicalRiskParity",
    "Linkage",
    "MaximumDiversification",
    "MaximumSharpe",
    "MeanVariance",
    "Method",
    "MinimumCvar",
    "MinimumVariance",
    "Objective",
    "Problem",
    "ProposedConversion",
    "ProposedTrade",
    "Rebalance",
    "RebalanceOptions",
    "RiskBudgeting",
    "Side",
    "Solution",
    "SolveStatus",
    "TargetReturn",
    "TargetVolatility",
    "TrackingErrorLimit",
    "Turnover",
    "transactions",
]


class Problem(_types.Problem):
    """A portfolio optimisation problem: the assets, estimates of their risk and return, and the
    constraints. The covariance matrix and expected returns should be in the same units, for
    example both annualised; scenario returns, for minimum CVaR, are per period."""

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

    def solve(self, objective: Objective) -> Solution:
        """The optimal portfolio for `objective`. A problem with no solution, such as one whose
        constraints cannot all hold, returns a solution whose ``status`` says why and whose
        ``weights`` are None."""
        return loads(Solution, call(_finalytics.solve, dumps(self), dumps(objective)))

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


def transactions(plan: Rebalance, prefix: str = "rebalance-") -> list[Transaction]:
    """A rebalancing proposal as transactions on its date (sales naming their lots, then
    conversions, then purchases), ready to add to the portfolio."""
    return loads(list[Transaction], call(_finalytics.rebalance_transactions, dumps(plan), prefix))
