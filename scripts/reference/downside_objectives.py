"""Generate reference values for the downside objectives and the concentration limit of
finalytics-opt.

Each problem is formulated afresh in CVXPY and compared in
``crates/finalytics-opt/tests/downside.rs`` with the result of the Rust formulation:

- minimum semivariance as the mean square of ``cp.neg`` of the portfolio's returns;
- minimum CDaR as the linear programme of Chekhlov, Uryasev and Zabarankin (2005) on the
  cumulative sum of the returns, its value checked against the drawdowns computed directly;
- minimum CVaR as the linear programme of Rockafellar and Uryasev (2000);
- the smallest effective number of assets as ``cp.norm(w, 2) <= 1 / sqrt(k)``; for the maximum
  Sharpe ratio under it, the reference searches the constrained frontier for the largest ratio
  by golden sections rather than transforming the problem.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/downside_objectives.py

The output, ``crates/finalytics-opt/tests/fixtures/downside.json``, is committed.
"""

import json
import math
import pathlib

import cvxpy as cp
import numpy as np

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics-opt/tests/fixtures/downside.json"


def data():
    rng = np.random.default_rng(20261104)
    n, t = 6, 400
    loadings = rng.uniform(0.3, 1.2, (n, 1))
    market = 0.01 * rng.standard_t(5, (t, 1))
    drift = rng.uniform(0.0001, 0.0006, n)
    scenarios = market @ loadings.T + 0.008 * rng.standard_t(4, (t, n)) + drift
    labels = [f"asset {i}" for i in range(n)]
    cov = np.cov(scenarios, rowvar=False) * 252
    # Expected returns are an input of their own, as a user may give them, rising with risk.
    mu = 0.03 + 0.4 * np.sqrt(np.diag(cov)) * rng.uniform(0.2, 1.0, n)
    return labels, cov, mu, scenarios


def solve(objective, constraints, solver=cp.CLARABEL, tolerance=1e-10):
    problem = cp.Problem(objective, constraints)
    if solver == cp.CLARABEL:
        problem.solve(solver=solver, tol_gap_abs=tolerance, tol_gap_rel=tolerance, tol_feas=tolerance)
    else:
        problem.solve(solver=solver)
    assert problem.status == cp.OPTIMAL, problem.status
    return problem


def drawdown_at_risk(returns, alpha):
    """The Rockafellar-Uryasev tail mean of the drawdowns of the cumulative returns, from a
    start of zero, computed directly."""
    cumulative = np.cumsum(returns)
    peaks = np.maximum.accumulate(np.concatenate([[0.0], cumulative]))[1:]
    falls = np.sort(peaks - cumulative)
    t = len(falls)
    best = math.inf
    for k in range(t):
        z = falls[k]
        best = min(best, z + np.maximum(falls - z, 0.0).sum() / (alpha * t))
    return best


def main():
    labels, cov, mu, scenarios = data()
    n, t = len(labels), len(scenarios)
    w = cp.Variable(n)
    psd = cp.psd_wrap(cov)
    long_only = [cp.sum(w) == 1, w >= 0, w <= 1]
    cases = []

    def case(name, objective, constraints, weights, value, unique):
        cases.append(
            {
                "name": name,
                "problem": objective,
                "constraints": constraints,
                "weights": [float(x) for x in weights],
                "objective_value": float(value),
                "unique": unique,
            }
        )

    floor = float(np.quantile(mu, 0.6))

    problem = solve(cp.Minimize(cp.sum_squares(cp.neg(scenarios @ w)) / t), long_only)
    case("minimum semivariance", {"type": "minimum_semivariance"}, {}, w.value, problem.value, True)

    problem = solve(
        cp.Minimize(cp.sum_squares(cp.neg(scenarios @ w)) / t), long_only + [mu @ w >= floor]
    )
    case(
        "minimum semivariance with a floor on expected return",
        {"type": "minimum_semivariance"},
        {"min_return": floor},
        w.value,
        problem.value,
        True,
    )

    for confidence, extra, constraints, name in [
        (0.95, [], {}, "minimum CDaR"),
        (
            0.9,
            [w[0] <= 0.25, mu @ w >= floor],
            {"asset_bounds": [{"asset": labels[0], "min": 0.0, "max": 0.25}], "min_return": floor},
            "minimum CDaR at 90% with an asset bound and a floor on expected return",
        ),
    ]:
        alpha = 1.0 - confidence
        cumulative = cp.cumsum(scenarios @ w)
        peaks = cp.Variable(t)
        z = cp.Variable()
        excess = cp.Variable(t)
        problem = solve(
            cp.Minimize(z + cp.sum(excess) / (alpha * t)),
            long_only
            + extra
            + [
                peaks >= cumulative,
                peaks[1:] >= peaks[:-1],
                peaks[0] >= 0,
                excess >= peaks - cumulative - z,
                excess >= 0,
            ],
            cp.HIGHS,
        )
        direct = drawdown_at_risk(scenarios @ w.value, alpha)
        assert abs(direct - problem.value) < 1e-8, (direct, problem.value)
        case(name, {"type": "minimum_cdar", "confidence": confidence}, constraints, w.value, direct, False)

    alpha = 0.05
    z = cp.Variable()
    excess = cp.Variable(t)
    problem = solve(
        cp.Minimize(z + cp.sum(excess) / (alpha * t)),
        long_only + [excess >= -scenarios @ w - z, excess >= 0, mu @ w >= floor],
        cp.HIGHS,
    )
    case(
        "minimum CVaR with a floor on expected return",
        {"type": "minimum_cvar", "confidence": 0.95},
        {"min_return": floor},
        w.value,
        problem.value,
        False,
    )

    concentrated = long_only + [cp.norm(w, 2) <= 1.0 / math.sqrt(4.5)]
    problem = solve(cp.Minimize(cp.quad_form(w, psd)), concentrated)
    case(
        "minimum variance with at least 4.5 effective assets",
        {"type": "minimum_variance"},
        {"min_effective_assets": 4.5},
        w.value,
        problem.value,
        True,
    )

    # The largest Sharpe ratio on the frontier of portfolios with at least 4 effective assets,
    # by golden sections on the expected return. The limit binds along the whole frontier, and
    # the largest ratio lies inside it, away from its ends.
    rf = 0.02
    spread = long_only + [cp.norm(w, 2) <= 1.0 / math.sqrt(4.0)]

    def frontier(target):
        # Clarabel cannot always reach 1e-10 on these, with two constraints binding.
        solve(cp.Minimize(cp.quad_form(w, psd)), spread + [mu @ w >= target], tolerance=1e-9)
        weights = w.value.copy()
        return (mu @ weights - rf) / math.sqrt(weights @ cov @ weights), weights

    low = solve(cp.Minimize(cp.quad_form(w, psd)), spread)
    low = float(mu @ w.value)
    high = float(solve(cp.Maximize(mu @ w), spread).value)
    ratio = (math.sqrt(5.0) - 1.0) / 2.0
    a, b = low, high
    c, d = b - ratio * (b - a), a + ratio * (b - a)
    fc, fd = frontier(c)[0], frontier(d)[0]
    while b - a > 1e-10:
        if fc > fd:
            b, d, fd = d, c, fc
            c = b - ratio * (b - a)
            fc = frontier(c)[0]
        else:
            a, c, fc = c, d, fd
            d = a + ratio * (b - a)
            fd = frontier(d)[0]
    sharpe, weights = frontier(0.5 * (a + b))
    case(
        "maximum Sharpe with at least 4 effective assets",
        {"type": "maximum_sharpe", "risk_free_rate": rf},
        {"min_effective_assets": 4.0},
        weights,
        sharpe,
        True,
    )

    fixture = {
        "generated_by": "scripts/reference/downside_objectives.py",
        "labels": labels,
        "covariance": cov.ravel().tolist(),
        "expected_returns": mu.tolist(),
        "scenarios": scenarios.T.tolist(),
        "cases": cases,
    }
    OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
