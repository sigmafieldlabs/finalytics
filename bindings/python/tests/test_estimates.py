"""Black–Litterman, robust and resampled optimisation against the shared reference fixtures."""

import json

import numpy as np
import pytest

import finalytics as fl
from finalytics._convert import loads

from conftest import FIXTURES

BLACK_LITTERMAN = json.loads((FIXTURES / "finalytics-opt/tests/fixtures/black_litterman.json").read_text())
OPTIMISATION = json.loads((FIXTURES / "finalytics-opt/tests/fixtures/optimisation.json").read_text())


def covariance():
    labels = BLACK_LITTERMAN["labels"]
    return fl.risk.matrix(np.array(BLACK_LITTERMAN["covariance"]).reshape(len(labels), len(labels)), labels)


@pytest.mark.parametrize("case", BLACK_LITTERMAN["cases"], ids=lambda c: c["name"])
def test_black_litterman_matches_pyportfolioopt(case):
    prior = loads(fl.opt.Prior, json.dumps(case["prior"]))
    views = loads(list[fl.opt.View], json.dumps(case["views"]))
    estimate = fl.opt.black_litterman(covariance(), views, prior=prior, tau=case["tau"])
    np.testing.assert_allclose(estimate.posterior_returns, case["posterior_returns"], rtol=0, atol=1e-14)
    np.testing.assert_allclose(estimate.posterior_covariance.values, case["posterior_covariance"], rtol=0, atol=1e-15)
    np.testing.assert_allclose(estimate.view_variances, case["view_variances"], rtol=0, atol=1e-16)


def test_market_weights_by_label():
    case = BLACK_LITTERMAN["cases"][0]
    labels = BLACK_LITTERMAN["labels"]
    weights = dict(zip(labels, case["prior"]["weights"]))
    views = [fl.opt.View(portfolio={"a0": 1.0}, expected_return=0.08), fl.opt.View(portfolio={"a3": 1.0}, expected_return=0.03)]
    estimate = fl.opt.black_litterman(covariance(), views, market_weights=weights, risk_free_rate=0.02)
    np.testing.assert_allclose(estimate.posterior_returns, case["posterior_returns"], rtol=0, atol=1e-14)
    with pytest.raises(ValueError, match="a5"):
        fl.opt.black_litterman(covariance(), views, market_weights={"a0": 1.0})


def test_implied_risk_aversion():
    f = BLACK_LITTERMAN["implied_risk_aversion"]
    found = fl.opt.implied_risk_aversion(f["returns"], f["periods_per_year"], f["risk_free_rate"])
    assert found == pytest.approx(f["risk_aversion"], rel=1e-12)


def problem() -> fl.opt.Problem:
    labels = OPTIMISATION["labels"]
    n = len(labels)
    return fl.opt.Problem.from_covariance(
        np.array(OPTIMISATION["covariance"]).reshape(n, n),
        labels=labels,
        expected_returns=OPTIMISATION["expected_returns"],
    )


@pytest.mark.parametrize("case", OPTIMISATION["robust"], ids=lambda c: c["name"])
def test_robust_mean_variance_matches_cvxpy(case):
    objective = loads(fl.opt.Objective, json.dumps(case["problem"]))
    constraints = loads(fl.opt.Constraints, json.dumps(case["constraints"]))
    base = problem()
    solution = fl.opt.Problem(
        labels=base.labels, covariance=base.covariance, expected_returns=base.expected_returns, constraints=constraints
    ).solve(objective)
    assert solution.status == "optimal"
    assert solution.objective_value == pytest.approx(case["objective_value"], rel=1e-7, abs=1e-7)
    np.testing.assert_allclose(solution.weights, case["weights"], atol=1e-4)


def test_resampling_matches_the_reference():
    case = OPTIMISATION["resampled"]
    constraints = loads(fl.opt.Constraints, json.dumps(case["constraints"]))
    base = problem()
    resampling = case["resampling"]
    found = fl.opt.Problem(
        labels=base.labels, covariance=base.covariance, expected_returns=base.expected_returns, constraints=constraints
    ).resampled(
        fl.opt.MeanVariance(risk_aversion=4.0),
        observations=resampling["observations"],
        samples=resampling["samples"],
        seed=resampling["seed"],
    )
    assert (found.solved, found.failed) == (resampling["samples"], 0)
    np.testing.assert_allclose(found.weights, case["weights"], atol=1e-4)
    np.testing.assert_allclose(found.weight_deviations, case["weight_deviations"], atol=1e-4)
