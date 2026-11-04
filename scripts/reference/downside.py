"""Generate reference values for the downside measures of finalytics-risk.

The conditional drawdown at risk is solved as the linear programme of Chekhlov, Uryasev and
Zabarankin with SciPy, the drawdowns and the Ulcer index come from pandas, the downside and
upside betas from ordinary least squares on the periods the benchmark fell or rose, and the
capture ratios are those of empyrical (compound annual returns of those periods). Each asset's
contribution to expected shortfall is checked by the Euler property: its weight times the
derivative of the portfolio's expected shortfall, found by central differences of the
Rockafellar-Uryasev linear programme, so that it shares no code with the closed form.

The returns are those of ``statistics.json``, written by ``risk.py``.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/downside.py

The output, ``downside.json`` in ``crates/finalytics-risk/tests/fixtures``, is committed.
"""

import json
import pathlib

import numpy as np
import pandas as pd
from scipy.optimize import linprog

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "crates/finalytics-risk/tests/fixtures"
CONFIDENCE = 0.95


def tail_mean(losses, alpha):
    """min over z of z + sum(max(loss - z, 0)) / (alpha T), as a linear programme."""
    t = len(losses)
    # Variables: z, then the excesses u_t >= 0 with u_t >= loss_t - z.
    c = np.concatenate([[1.0], np.full(t, 1.0 / (alpha * t))])
    a = np.hstack([-np.ones((t, 1)), -np.eye(t)])
    bounds = [(None, None)] + [(0, None)] * t
    result = linprog(c, A_ub=a, b_ub=-np.asarray(losses), bounds=bounds, method="highs")
    assert result.status == 0, result.message
    return float(result.fun)


def drawdowns(returns):
    value = pd.Series(np.concatenate([[1.0], np.cumprod(1.0 + np.asarray(returns))]))
    return (value / value.cummax() - 1.0).to_numpy()[1:]


def regression_slope(y, x):
    x = np.column_stack([np.ones(len(x)), x])
    coefficients, *_ = np.linalg.lstsq(x, y, rcond=None)
    return float(coefficients[1])


def annual_return(returns, periods):
    years = len(returns) / periods
    return float(np.prod(1.0 + returns) ** (1.0 / years) - 1.0)


def relative(returns, benchmark, periods):
    out = {}
    for side, mask in (("downside", benchmark < 0.0), ("upside", benchmark > 0.0)):
        r, b = returns[mask], benchmark[mask]
        out[f"{side}_beta"] = regression_slope(r, b)
        out[f"{side}_capture"] = annual_return(r, periods) / annual_return(b, periods)
    return out


def contributions(scenarios, weights):
    """Each asset's weight times the derivative of the expected shortfall in its weight."""
    def shortfall(w):
        return tail_mean(-(scenarios @ w), 1.0 - CONFIDENCE)

    step = 1e-6
    out = []
    for i in range(len(weights)):
        up, down = weights.copy(), weights.copy()
        up[i] += step
        down[i] -= step
        out.append(float(weights[i] * (shortfall(up) - shortfall(down)) / (2.0 * step)))
    return shortfall(weights), out


def main():
    source = json.loads((FIXTURES / "statistics.json").read_text())
    cases = []
    for case in source["cases"]:
        returns = np.asarray(case["returns"])
        benchmark = np.asarray(case["benchmark"])
        periods = case["options"]["periods_per_year"]
        falls = drawdowns(returns)
        cases.append(
            {
                "name": case["name"],
                "conditional_drawdown_at_risk": -tail_mean(-falls, 1.0 - CONFIDENCE),
                "ulcer_index": float(np.sqrt(np.mean(falls**2))),
                "relative": relative(returns, benchmark, periods),
            }
        )
    rng = np.random.default_rng(20261104)
    scenarios = 0.01 * rng.standard_t(5, (500, 4)) + np.array([0.0004, 0.0002, 0.0001, 0.0003])
    weights = np.array([0.4, 0.3, 0.2, 0.1])
    shortfall, parts = contributions(scenarios, weights)
    content = {
        "generated_by": "scripts/reference/downside.py",
        "confidence": CONFIDENCE,
        "cases": cases,
        "shortfall": {
            "labels": ["a", "b", "c", "d"],
            "scenarios": scenarios.T.tolist(),
            "weights": weights.tolist(),
            "expected_shortfall": shortfall,
            "contributions": parts,
        },
    }
    path = FIXTURES / "downside.json"
    path.write_text(json.dumps(content, indent=1) + "\n")
    print(f"wrote {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
