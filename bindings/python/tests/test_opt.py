import json

import numpy as np
import pytest

import finalytics as fl
from finalytics import opt

from conftest import FIXTURES

FIXTURE = json.loads((FIXTURES / "finalytics-opt/tests/fixtures/optimisation.json").read_text())


def problem() -> opt.Problem:
    n = len(FIXTURE["labels"])
    covariance = np.array(FIXTURE["covariance"]).reshape(n, n)
    return opt.Problem.from_covariance(
        covariance,
        labels=FIXTURE["labels"],
        expected_returns=FIXTURE["expected_returns"],
        scenarios=np.array(FIXTURE["scenarios"]).T,
    )


@pytest.mark.parametrize("case", FIXTURE["cases"], ids=lambda c: c["name"])
def test_solutions_match_the_reference(case):
    objective = fl._convert.from_json(opt.Objective, case["problem"])
    constraints = fl._convert.from_json(opt.Constraints, case["constraints"])
    base = problem()
    solution = opt.Problem(
        labels=base.labels,
        covariance=base.covariance,
        expected_returns=base.expected_returns,
        scenarios=base.scenarios,
        constraints=constraints,
    ).solve(objective)
    assert solution.status == "optimal"
    assert solution.objective_value == pytest.approx(case["objective_value"], rel=1e-7, abs=1e-7)
    if not isinstance(objective, opt.MinimumCvar):
        np.testing.assert_allclose(solution.weights, case["weights"], atol=2e-5)


def test_objectives_and_status():
    p = problem()
    sharpe = p.solve(opt.MaximumSharpe(risk_free_rate=0.03))
    assert sharpe.method == "interior_point" and sharpe.max_violation < 1e-7
    parity = p.solve(opt.RiskBudgeting())
    assert parity.method == "newton" and parity.objective_value < 1e-12
    hrp = p.solve(opt.HierarchicalRiskParity(linkage="ward"))
    assert hrp.objective_value is None and sum(hrp.weights) == pytest.approx(1.0)
    tight = opt.Problem(
        labels=p.labels, covariance=p.covariance, constraints=opt.Constraints(bounds=opt.Bound(min=0, max=0.1))
    )
    infeasible = tight.solve(opt.MinimumVariance())
    assert infeasible.status == "infeasible" and infeasible.weights is None
    frontier = p.frontier(6)
    assert len(frontier) == 6 and frontier[0].volatility < frontier[-1].volatility
    with pytest.raises(fl.ValidationError) as caught:
        p.solve(opt.MeanVariance(risk_aversion=-1))
    assert [i.path for i in caught.value.issues] == ["objective.risk_aversion"]
