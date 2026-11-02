import json

import numpy as np
import pandas as pd
import pytest

import finalytics as fl

from conftest import FIXTURES

FACTORS = json.loads((FIXTURES / "finalytics-risk/tests/fixtures/factors.json").read_text())
OPTIMISATION = json.loads((FIXTURES / "finalytics-opt/tests/fixtures/optimisation.json").read_text())


def returns():
    return np.array(FACTORS["returns"]).T


def test_regression_model_matches_statsmodels():
    factors = pd.DataFrame(np.array(FACTORS["factor_returns"]).T, columns=FACTORS["factors"])
    estimate = fl.risk.regression_model(returns(), factors, labels=FACTORS["assets"])
    expected = FACTORS["regression"]
    assert estimate.method == "regression"
    assert estimate.model.factor_covariance.labels == FACTORS["factors"]
    np.testing.assert_allclose(estimate.model.exposures, expected["exposures"], rtol=0, atol=1e-11 * 2)
    np.testing.assert_allclose([f.intercept for f in estimate.fit], expected["intercepts"], rtol=0, atol=1e-14)
    np.testing.assert_allclose([f.r_squared for f in estimate.fit], expected["r_squared"], rtol=1e-12)
    np.testing.assert_allclose(estimate.model.specific_variances, expected["specific_variances"], rtol=1e-11)


def test_statistical_model_matches_scikit_learn():
    estimate = fl.risk.statistical_model(returns(), FACTORS["components"])
    expected = FACTORS["statistical"]
    assert estimate.model.factor_covariance.labels == ["pc1", "pc2", "pc3", "pc4"]
    np.testing.assert_allclose(estimate.explained_variance, expected["explained_variance"], rtol=1e-11)
    np.testing.assert_allclose(estimate.model.exposures, expected["exposures"], rtol=0, atol=1e-11)


def test_fundamental_model_takes_exposures_as_a_frame():
    exposures = pd.DataFrame(FACTORS["exposures"], index=FACTORS["assets"], columns=FACTORS["factors"])
    estimate = fl.risk.fundamental_model(returns(), exposures, labels=FACTORS["assets"])
    expected = FACTORS["fundamental"]
    assert estimate.model.factor_covariance.labels == FACTORS["factors"]
    np.testing.assert_allclose(estimate.factor_returns, expected["factor_returns"], rtol=0, atol=1e-14)
    np.testing.assert_allclose(estimate.model.specific_variances, expected["specific_variances"], rtol=1e-11)


def reference_model() -> fl.risk.FactorModel:
    expected = FACTORS["regression"]
    exposures = pd.DataFrame(expected["exposures"], index=FACTORS["assets"], columns=FACTORS["factors"])
    covariance = np.array(expected["factor_covariance"]).reshape(3, 3)
    return fl.risk.factor_model(exposures, covariance, expected["specific_variances"])


def test_factor_risk_matches_its_definition():
    model = reference_model()
    risk = fl.risk.factor_risk(FACTORS["weights"], model)
    expected = FACTORS["risk"]
    assert risk.volatility == pytest.approx(expected["volatility"], rel=1e-13)
    assert [f.component for f in risk.factors] == pytest.approx(expected["factor_components"], rel=1e-12)
    assert sum(f.component for f in risk.factors) + risk.specific_component == pytest.approx(risk.volatility)
    active = fl.risk.factor_tracking_error(FACTORS["weights"], FACTORS["benchmark"], model)
    assert active.volatility == pytest.approx(FACTORS["tracking_error"]["volatility"], rel=1e-13)
    implied = fl.risk.to_array(fl.risk.implied_covariance(model))
    np.testing.assert_allclose(implied.ravel(), FACTORS["risk"]["covariance"], rtol=1e-14)


def test_values_cross_the_boundary_exactly():
    # With one-hot weights, each exposure B' w is a single product with 1.0 and sums with 0.0,
    # so it equals the model's entry bit for bit only if the number was read exactly.
    model = reference_model()
    n = len(model.assets)
    for i in (0, 5, n - 1):
        weights = np.zeros(n)
        weights[i] = 1.0
        exposures = [f.exposure for f in fl.risk.factor_risk(weights, model).factors]
        assert exposures == model.exposures[i]


def test_invalid_models_are_reported():
    with pytest.raises(fl.ValidationError) as caught:
        fl.risk.factor_model([[1.0], [0.5]], [[0.04]], [0.01, -0.02])
    assert [i.path for i in caught.value.issues] == ["specific_variances[1]"]


def test_scaled_annualises_a_model():
    model = reference_model()
    annual = fl.risk.scaled(model, 252)
    assert annual.exposures == model.exposures
    assert annual.specific_variances[0] == pytest.approx(252 * model.specific_variances[0])


def test_optimisation_with_a_factor_model():
    block = OPTIMISATION["factor_model"]
    model = fl.risk.factor_model(
        np.array(block["exposures"]),
        np.array(block["factor_covariance"]).reshape(2, 2),
        block["specific_variances"],
        assets=OPTIMISATION["labels"],
        factors=["f0", "f1"],
    )
    case = block["cases"][0]
    limits = [fl.opt.FactorExposureLimit(**limit) for limit in case["constraints"]["factor_exposures"]]
    problem = fl.opt.Problem.from_factor_model(
        model,
        expected_returns=OPTIMISATION["expected_returns"],
        constraints=fl.opt.Constraints(factor_exposures=limits),
    )
    solution = problem.solve(fl.opt.MinimumVariance())
    assert solution.status == "optimal"
    assert solution.objective_value == pytest.approx(case["objective_value"], rel=1e-7)
    np.testing.assert_allclose(solution.weights, case["weights"], atol=1e-4)
