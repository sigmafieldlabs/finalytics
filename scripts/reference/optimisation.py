"""Generate reference values for the finalytics-opt tests.

Each problem is formulated afresh in CVXPY, solved to tight tolerances, and compared in
``crates/finalytics-opt/tests/reference.rs`` with the result of the Rust formulation. The maximum
Sharpe ratio portfolio and hierarchical risk parity come from PyPortfolioOpt. The factor model
cases are formulated in CVXPY in factor form, with the factor exposures as variables. The robust
cases are formulated in CVXPY with the worst-case expected return written out, and the resampled
case reimplements the engine's random numbers (SplitMix64 and Box-Muller) and simulation, and
solves every simulated problem in CVXPY.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/optimisation.py

The output, ``crates/finalytics-opt/tests/fixtures/optimisation.json``, is committed. Regenerate it
only when a case is added or a reference library changes, and review the difference.
"""

import json
import math
import pathlib

import cvxpy as cp
import numpy as np
import pandas as pd
import scipy.cluster.hierarchy as sch
from pypfopt import EfficientFrontier, HRPOpt
from scipy import stats

# PyPortfolioOpt 1.6 validates the linkage method against a private SciPy table that SciPy 1.18
# no longer has.
if not hasattr(sch, "_LINKAGE_METHODS"):
    sch._LINKAGE_METHODS = {"single", "complete", "average", "weighted", "centroid", "median", "ward"}

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics-opt/tests/fixtures/optimisation.json"
TIGHT = {"tol_gap_abs": 1e-10, "tol_gap_rel": 1e-10, "tol_feas": 1e-10}


def data():
    rng = np.random.default_rng(20261104)
    n = 8
    loadings = rng.uniform(0.2, 1.3, (n, 2))
    factor_cov = np.array([[0.03, 0.004], [0.004, 0.01]])
    specific = rng.uniform(0.01, 0.04, n)
    cov = loadings @ factor_cov @ loadings.T + np.diag(specific)
    mu = 0.02 + 0.6 * np.sqrt(np.diag(cov)) * rng.uniform(0.2, 1.0, n)
    scenarios = rng.multivariate_normal(mu / 252, cov / 252, 300)
    labels = [f"asset {i}" for i in range(n)]
    return labels, cov, mu, scenarios, (loadings, factor_cov, specific)


def solve(objective, constraints):
    problem = cp.Problem(objective, constraints)
    problem.solve(solver=cp.CLARABEL, **TIGHT)
    assert problem.status == cp.OPTIMAL, problem.status
    return problem


def factor_model_cases(labels, mu, model):
    """Problems on the factor model behind the covariance matrix, in factor form."""
    loadings, factor_cov, specific = model
    n, k = loadings.shape
    w = cp.Variable(n)
    y = cp.Variable(k)
    risk = cp.quad_form(y, factor_cov) + cp.sum(cp.multiply(specific, cp.square(w)))
    exposure = [y == loadings.T @ w]
    long_only = [cp.sum(w) == 1, w >= 0, w <= 1]
    equal = np.full(n, 1.0 / n)
    active = loadings.T @ equal
    cases = []

    def variance(x):
        z = loadings.T @ x
        return z @ factor_cov @ z + specific @ x**2

    def case(name, problem, constraints, weights, value):
        cases.append(
            {
                "name": name,
                "problem": problem,
                "constraints": constraints,
                "weights": [float(x) for x in weights],
                "objective_value": float(value),
            }
        )

    limits = [
        {"factor": "f0", "min": 0.75, "max": 0.8},
        {"factor": "f1", "min": -0.05, "max": 0.05, "benchmark": equal.tolist()},
    ]
    solve(
        cp.Minimize(risk),
        exposure + long_only + [y[0] >= 0.75, y[0] <= 0.8, cp.abs(y[1] - active[1]) <= 0.05],
    )
    case(
        "minimum variance with factor exposure limits",
        {"objective": "minimum_variance"},
        {"factor_exposures": limits},
        w.value,
        variance(w.value),
    )

    tracking_variance = cp.quad_form(y - active, factor_cov) + cp.sum(
        cp.multiply(specific, cp.square(w - equal))
    )
    solve(
        cp.Maximize(mu @ w - 1.5 * risk),
        exposure + long_only + [tracking_variance <= 0.04**2],
    )
    case(
        "mean-variance with a tracking-error limit",
        {"objective": "mean_variance", "risk_aversion": 3.0},
        {"tracking_error": {"benchmark": equal.tolist(), "limit": 0.04}},
        w.value,
        mu @ w.value - 1.5 * variance(w.value),
    )

    t = cp.Variable(n)
    s = cp.Variable()
    z = cp.Variable(k)
    solve(
        cp.Minimize(cp.quad_form(z, factor_cov) + cp.sum(cp.multiply(specific, cp.square(t)))),
        [z == loadings.T @ t, (mu - 0.03) @ t == 1, cp.sum(t) == s, t >= 0, t <= s],
    )
    weights = t.value / s.value
    case(
        "maximum Sharpe",
        {"objective": "maximum_sharpe", "risk_free_rate": 0.03},
        {},
        weights,
        (mu @ weights - 0.03) / np.sqrt(variance(weights)),
    )

    solve(cp.Maximize(mu @ w), exposure + long_only + [risk <= 0.2**2])
    case(
        "target volatility",
        {"objective": "target_volatility", "target": 0.2},
        {},
        w.value,
        mu @ w.value,
    )
    return cases


def robust_cases(labels, cov, mu):
    """Robust mean-variance problems, with the worst-case expected return written out."""
    n = len(labels)
    w = cp.Variable(n)
    psd = cp.psd_wrap(cov)
    long_only = [cp.sum(w) == 1, w >= 0, w <= 1]
    root = np.linalg.cholesky(cov).T
    cases = []

    def case(name, uncertainty, constraints, extra, shortfall):
        problem = cp.Problem(cp.Maximize(mu @ w - shortfall(w) - 2.0 * cp.quad_form(w, psd)), long_only + extra)
        problem.solve(solver=cp.CLARABEL, **TIGHT)
        assert problem.status == cp.OPTIMAL, problem.status
        x = w.value
        cases.append(
            {
                "name": name,
                "problem": {"objective": "robust_mean_variance", "risk_aversion": 4.0, "uncertainty": uncertainty},
                "constraints": constraints,
                "weights": x.tolist(),
                "objective_value": float(problem.value),
            }
        )

    case(
        "ellipsoid of the covariance",
        {"set": "ellipsoid", "size": 0.3},
        {},
        [],
        lambda x: 0.3 * cp.norm(root @ x),
    )
    observations = 60
    error = np.diag(np.diag(cov)) / observations
    size = math.sqrt(stats.chi2.ppf(0.95, n))
    error_root = np.sqrt(np.diag(error))
    case(
        "ellipsoid of the estimation error, with a group",
        {"set": "ellipsoid", "size": size, "covariance": {"labels": labels, "values": error.ravel().tolist()}},
        {"groups": [{"name": "first three", "assets": labels[:3], "min": 0.0, "max": 0.5}]},
        [cp.sum(w[:3]) <= 0.5],
        lambda x: size * cp.norm(cp.multiply(error_root, x)),
    )
    half_widths = 0.5 * np.sqrt(np.diag(cov)) / math.sqrt(observations)
    case(
        "box",
        {"set": "box", "half_widths": half_widths.tolist()},
        {},
        [],
        lambda x: half_widths @ cp.abs(x),
    )
    return cases


class Normal:
    """The engine's standard normal numbers: SplitMix64 and the Box-Muller transform."""

    MASK = (1 << 64) - 1

    def __init__(self, seed):
        self.state = seed
        self.spare = None

    def next_u64(self):
        self.state = (self.state + 0x9E3779B97F4A7C15) & self.MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & self.MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & self.MASK
        return z ^ (z >> 31)

    def uniform(self):
        return ((self.next_u64() >> 11) + 0.5) / float(1 << 53)

    def next(self):
        if self.spare is not None:
            z, self.spare = self.spare, None
            return z
        radius = math.sqrt(-2.0 * math.log(self.uniform()))
        angle = 2.0 * math.pi * self.uniform()
        self.spare = radius * math.sin(angle)
        return radius * math.cos(angle)


def resampled_case(labels, cov, mu):
    """Michaud resampling of a mean-variance problem with a group limit."""
    n = len(labels)
    samples, observations, seed = 30, 60, 2026
    lower = np.linalg.cholesky(cov)
    normal = Normal(seed)
    w = cp.Variable(n)
    solutions = []
    for _ in range(samples):
        history = np.empty((observations, n))
        for t in range(observations):
            z = np.array([normal.next() for _ in range(n)])
            history[t] = mu + lower @ z
        means = history.mean(axis=0)
        sample = np.cov(history, rowvar=False, ddof=1)
        problem = cp.Problem(
            cp.Maximize(means @ w - 2.0 * cp.quad_form(w, cp.psd_wrap(sample))),
            [cp.sum(w) == 1, w >= 0, w <= 1, cp.sum(w[:3]) <= 0.5],
        )
        problem.solve(solver=cp.CLARABEL, **TIGHT)
        assert problem.status == cp.OPTIMAL, problem.status
        solutions.append(w.value.copy())
    solutions = np.array(solutions)
    return {
        "problem": {"objective": "mean_variance", "risk_aversion": 4.0},
        "constraints": {"groups": [{"name": "first three", "assets": labels[:3], "min": 0.0, "max": 0.5}]},
        "resampling": {"samples": samples, "observations": observations, "seed": seed},
        "weights": solutions.mean(axis=0).tolist(),
        "weight_deviations": solutions.std(axis=0, ddof=1).tolist(),
    }


def main():
    labels, cov, mu, scenarios, model = data()
    n = len(labels)
    w = cp.Variable(n)
    psd = cp.psd_wrap(cov)
    long_only = [cp.sum(w) == 1, w >= 0, w <= 1]
    equal = np.full(n, 1.0 / n)
    cases = []

    def case(name, problem, constraints, weights, value):
        cases.append(
            {
                "name": name,
                "problem": problem,
                "constraints": constraints,
                "weights": [float(x) for x in weights],
                "objective_value": float(value),
            }
        )

    solve(cp.Minimize(cp.quad_form(w, psd)), long_only)
    case("minimum variance", {"objective": "minimum_variance"}, {}, w.value, w.value @ cov @ w.value)

    groups = [{"name": "first three", "assets": labels[:3], "min": 0.0, "max": 0.4}]
    solve(
        cp.Maximize(mu @ w - 2.0 * cp.quad_form(w, psd)),
        long_only + [cp.sum(w[:3]) <= 0.4, w[7] <= 0.1],
    )
    case(
        "mean-variance with a group and an asset bound",
        {"objective": "mean_variance", "risk_aversion": 4.0},
        {"groups": groups, "asset_bounds": [{"asset": labels[7], "min": 0.0, "max": 0.1}]},
        w.value,
        mu @ w.value - 2.0 * w.value @ cov @ w.value,
    )

    solve(
        cp.Minimize(cp.quad_form(w, psd)),
        [cp.sum(w) == 1, w >= -0.2, w <= 0.6, cp.norm1(w) <= 1.6, mu @ w >= 0.16],
    )
    case(
        "long-short target return with a leverage limit",
        {"objective": "target_return", "target": 0.16},
        {"bounds": {"min": -0.2, "max": 0.6}, "leverage": 1.6},
        w.value,
        w.value @ cov @ w.value,
    )

    solve(
        cp.Maximize(mu @ w),
        long_only + [cp.quad_form(w, psd) <= 0.17**2, cp.norm1(w - equal) <= 0.5],
    )
    case(
        "target volatility with a turnover limit",
        {"objective": "target_volatility", "target": 0.17},
        {"turnover": {"current": equal.tolist(), "limit": 0.5}},
        w.value,
        mu @ w.value,
    )

    costs = np.full(n, 0.004)
    current = np.zeros(n)
    current[0] = current[1] = 0.5
    solve(
        cp.Maximize(mu @ w - costs @ cp.abs(w - current) - 1.0 * cp.quad_form(w, psd)),
        long_only + [cp.quad_form(w - equal, psd) <= 0.05**2],
    )
    value = mu @ w.value - 1.0 * w.value @ cov @ w.value
    case(
        "mean-variance with trading costs and a tracking-error limit",
        {"objective": "mean_variance", "risk_aversion": 2.0},
        {
            "turnover": {"current": current.tolist(), "costs": costs.tolist()},
            "tracking_error": {"benchmark": equal.tolist(), "limit": 0.05},
        },
        w.value,
        value,
    )

    frame = pd.Series(mu, index=labels)
    covariance = pd.DataFrame(cov, index=labels, columns=labels)
    ef = EfficientFrontier(frame, covariance, weight_bounds=(0, 1), solver="CLARABEL")
    ef.max_sharpe(risk_free_rate=0.03)
    weights = np.array([ef.weights[i] for i in range(n)])
    sharpe = (mu @ weights - 0.03) / np.sqrt(weights @ cov @ weights)
    case("maximum Sharpe", {"objective": "maximum_sharpe", "risk_free_rate": 0.03}, {}, weights, sharpe)

    ef = EfficientFrontier(frame, covariance, weight_bounds=(0, 0.3), solver="CLARABEL")
    ef.add_sector_constraints(
        {label: "first three" if i < 3 else "rest" for i, label in enumerate(labels)},
        {"first three": 0.2},
        {"first three": 0.5},
    )
    ef.max_sharpe(risk_free_rate=0.03)
    weights = np.array([ef.weights[i] for i in range(n)])
    sharpe = (mu @ weights - 0.03) / np.sqrt(weights @ cov @ weights)
    case(
        "maximum Sharpe with bounds and a group",
        {"objective": "maximum_sharpe", "risk_free_rate": 0.03},
        {
            "bounds": {"min": 0.0, "max": 0.3},
            "groups": [{"name": "first three", "assets": labels[:3], "min": 0.2, "max": 0.5}],
        },
        weights,
        sharpe,
    )

    sigma = np.sqrt(np.diag(cov))
    y = cp.Variable(n)
    k = cp.Variable()
    solve(cp.Minimize(cp.quad_form(y, psd)), [sigma @ y == 1, cp.sum(y) == k, y >= 0, y <= k])
    weights = y.value / k.value
    ratio = sigma @ weights / np.sqrt(weights @ cov @ weights)
    case("maximum diversification", {"objective": "maximum_diversification"}, {}, weights, ratio)

    alpha = 0.05
    z = cp.Variable()
    u = cp.Variable(len(scenarios))
    problem = cp.Problem(
        cp.Minimize(z + cp.sum(u) / (alpha * len(scenarios))),
        long_only + [u >= -scenarios @ w - z, u >= 0],
    )
    problem.solve(solver=cp.HIGHS)
    assert problem.status == cp.OPTIMAL, problem.status
    case(
        "minimum CVaR",
        {"objective": "minimum_cvar", "confidence": 0.95},
        {},
        w.value,
        problem.value,
    )

    # A second, larger matrix with several clusters, on which the linkage methods disagree.
    rng = np.random.default_rng(20261105)
    loadings = rng.normal(0, 1, (20, 4)) * rng.uniform(0.2, 1, (20, 4))
    clustered = loadings @ loadings.T * 0.01 + np.diag(rng.uniform(0.005, 0.03, 20))
    hrp = []
    for matrix in [cov, clustered]:
        names = [f"asset {i}" for i in range(len(matrix))]
        frame = pd.DataFrame(matrix, index=names, columns=names)
        results = []
        for linkage in ["single", "complete", "average", "ward"]:
            weights = HRPOpt(cov_matrix=frame).optimize(linkage_method=linkage)
            results.append({"linkage": linkage, "weights": [float(weights[n]) for n in names]})
        hrp.append({"covariance": matrix.ravel().tolist(), "results": results})

    factor_cases = factor_model_cases(labels, mu, model)
    robust = robust_cases(labels, cov, mu)
    resampled = resampled_case(labels, cov, mu)

    fixture = {
        "generated_by": "scripts/reference/optimisation.py",
        "labels": labels,
        "covariance": cov.ravel().tolist(),
        "expected_returns": mu.tolist(),
        "scenarios": scenarios.T.tolist(),
        "cases": cases,
        "hierarchical_risk_parity": hrp,
        "factor_model": {
            "exposures": model[0].tolist(),
            "factor_covariance": model[1].ravel().tolist(),
            "specific_variances": model[2].tolist(),
            "cases": factor_cases,
        },
        "robust": robust,
        "resampled": resampled,
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
