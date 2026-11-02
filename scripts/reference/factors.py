"""Generate reference values for the factor model tests.

Time-series regressions come from statsmodels, principal components from scikit-learn, and the
cross-sectional regressions of the fundamental model from NumPy's least squares, so that each
check in ``crates/finalytics-risk/tests/factors.rs`` compares against an independent
implementation. The factor risk decomposition is computed here from its definition.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/factors.py

The output, ``factors.json`` in ``crates/finalytics-risk/tests/fixtures``, is committed.
Regenerate it only when a case is added or a reference library changes, and review the
difference.
"""

import json
import pathlib

import numpy as np
import statsmodels.api as sm
from sklearn.decomposition import PCA

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "crates/finalytics-risk/tests/fixtures/factors.json"

PERIODS = 250
ASSETS = 12
COMPONENTS = 4


def data():
    rng = np.random.default_rng(20261102)
    factor_cov = np.array(
        [
            [1.0e-4, 1.5e-5, -1.0e-5],
            [1.5e-5, 4.0e-5, 5.0e-6],
            [-1.0e-5, 5.0e-6, 3.0e-5],
        ]
    )
    factors = rng.multivariate_normal([3e-4, 1e-4, 5e-5], factor_cov, PERIODS)
    betas = np.column_stack(
        [
            rng.uniform(0.6, 1.4, ASSETS),
            rng.uniform(-0.8, 0.8, ASSETS),
            rng.uniform(-0.6, 0.6, ASSETS),
        ]
    )
    alphas = rng.normal(0.0, 1e-4, ASSETS)
    specific = rng.uniform(0.004, 0.012, ASSETS)
    noise = rng.standard_normal((PERIODS, ASSETS)) * specific
    returns = alphas + factors @ betas.T + noise
    return returns, factors, betas


def regression(returns, factors):
    design = sm.add_constant(factors)
    fits = [sm.OLS(returns[:, i], design).fit() for i in range(returns.shape[1])]
    return {
        "intercepts": [float(f.params[0]) for f in fits],
        "exposures": [[float(x) for x in f.params[1:]] for f in fits],
        "specific_variances": [float(f.mse_resid) for f in fits],
        "r_squared": [float(f.rsquared) for f in fits],
        "factor_covariance": np.cov(factors, rowvar=False, ddof=1).ravel().tolist(),
    }


def statistical(returns):
    pca = PCA(n_components=COMPONENTS, svd_solver="full").fit(returns)
    components = pca.components_
    # The convention the engine documents: each component's largest loading is positive.
    for row in components:
        if row[np.argmax(np.abs(row))] < 0:
            raise AssertionError("scikit-learn's sign convention changed")
    sample = np.cov(returns, rowvar=False, ddof=1)
    explained = (components**2 * pca.explained_variance_[:, None]).sum(axis=0)
    return {
        "eigenvalues": pca.explained_variance_.tolist(),
        "explained_variance": pca.explained_variance_ratio_.tolist(),
        "exposures": components.T.tolist(),
        "specific_variances": (np.diag(sample) - explained).tolist(),
        "scores": pca.transform(returns).T.tolist(),
        "means": pca.mean_.tolist(),
    }


def fundamental(returns, exposures):
    estimated = np.linalg.lstsq(exposures, returns.T, rcond=None)[0].T
    residuals = returns - estimated @ exposures.T
    specific = residuals.var(axis=0, ddof=1)
    return {
        "factor_returns": estimated.T.tolist(),
        "factor_covariance": np.cov(estimated, rowvar=False, ddof=1).ravel().tolist(),
        "specific_variances": specific.tolist(),
        "r_squared": (1.0 - specific / returns.var(axis=0, ddof=1)).tolist(),
    }


def decomposition(weights, exposures, factor_cov, specific):
    x = exposures.T @ weights
    fx = factor_cov @ x
    variance = x @ fx + (specific * weights**2).sum()
    sigma = np.sqrt(variance)
    covariance = exposures @ factor_cov @ exposures.T + np.diag(specific)
    return {
        "volatility": float(sigma),
        "factor_volatility": float(np.sqrt(x @ fx)),
        "specific_volatility": float(np.sqrt((specific * weights**2).sum())),
        "factor_components": (x * fx / sigma).tolist(),
        "specific_component": float((specific * weights**2).sum() / sigma),
        "asset_components": (weights * (covariance @ weights) / sigma).tolist(),
        "covariance": covariance.ravel().tolist(),
    }


def main():
    returns, factors, _ = data()
    rng = np.random.default_rng(7)
    size = rng.standard_normal(ASSETS)
    value = rng.standard_normal(ASSETS)
    exposures = np.column_stack([np.ones(ASSETS), size, value])
    weights = rng.dirichlet(np.ones(ASSETS))
    benchmark = np.full(ASSETS, 1.0 / ASSETS)

    reg = regression(returns, factors)
    risk = decomposition(
        weights,
        np.array(reg["exposures"]),
        np.array(reg["factor_covariance"]).reshape(3, 3),
        np.array(reg["specific_variances"]),
    )
    active = decomposition(
        weights - benchmark,
        np.array(reg["exposures"]),
        np.array(reg["factor_covariance"]).reshape(3, 3),
        np.array(reg["specific_variances"]),
    )
    fixture = {
        "assets": [f"a{i}" for i in range(ASSETS)],
        "factors": ["market", "size", "value"],
        "returns": returns.T.tolist(),
        "factor_returns": factors.T.tolist(),
        "exposures": exposures.tolist(),
        "weights": weights.tolist(),
        "benchmark": benchmark.tolist(),
        "components": COMPONENTS,
        "regression": reg,
        "statistical": statistical(returns),
        "fundamental": fundamental(returns, exposures),
        "risk": risk,
        "tracking_error": active,
    }
    content = {"generated_by": "scripts/reference/factors.py", **fixture}
    FIXTURE.write_text(json.dumps(content, indent=1) + "\n")
    print(f"wrote {FIXTURE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
