"""Generate reference values for the Black-Litterman tests.

The prior returns, posterior returns, posterior covariance and view variances come from
PyPortfolioOpt's ``BlackLittermanModel`` and ``market_implied_prior_returns``, and the implied risk
aversion from ``market_implied_risk_aversion``; ``crates/finalytics-opt/tests/black_litterman.rs``
compares them with the Rust implementation.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/black_litterman.py

The output, ``crates/finalytics-opt/tests/fixtures/black_litterman.json``, is committed. Regenerate
it only when a case is added or a reference library changes, and review the difference.
"""

import json
import pathlib

import numpy as np
import pandas as pd
from pypfopt import black_litterman as bl

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics-opt/tests/fixtures/black_litterman.json"


def data():
    rng = np.random.default_rng(20261106)
    n = 6
    loadings = rng.uniform(0.3, 1.2, (n, 2))
    factor_cov = np.array([[0.035, 0.006], [0.006, 0.012]])
    cov = loadings @ factor_cov @ loadings.T + np.diag(rng.uniform(0.01, 0.05, n))
    caps = rng.uniform(50, 500, n)
    labels = [f"a{i}" for i in range(n)]
    return labels, cov, caps


def main():
    labels, cov, caps = data()
    covariance = pd.DataFrame(cov, index=labels, columns=labels)
    market = pd.Series(caps, index=labels)
    prior = bl.market_implied_prior_returns(market, 2.5, covariance, risk_free_rate=0.02)
    given = np.linspace(0.04, 0.09, len(labels))
    cases = []

    def case(name, prior_input, views, model):
        cases.append(
            {
                "name": name,
                "prior": prior_input,
                "views": views,
                "tau": model.tau,
                "prior_returns": np.asarray(model.pi).ravel().tolist(),
                "posterior_returns": model.bl_returns().tolist(),
                "posterior_covariance": model.bl_cov().to_numpy().ravel().tolist(),
                "view_variances": np.diag(model.omega).tolist(),
            }
        )

    market_prior = {"type": "market", "weights": caps.tolist(), "risk_aversion": 2.5, "risk_free_rate": 0.02}

    absolute = {"a0": 0.08, "a3": 0.03}
    case(
        "absolute views, proportional uncertainty",
        market_prior,
        [{"portfolio": {a: 1.0}, "expected_return": q} for a, q in absolute.items()],
        bl.BlackLittermanModel(covariance, pi=prior, absolute_views=absolute, tau=0.05),
    )

    # a1 outperforms a2 by 2%, and an equal-weighted view on a4 and a5.
    p = np.zeros((2, 6))
    p[0, 1], p[0, 2] = 1.0, -1.0
    p[1, 4], p[1, 5] = 0.5, 0.5
    q = np.array([0.02, 0.06])
    case(
        "relative and portfolio views",
        market_prior,
        [
            {"portfolio": {"a1": 1.0, "a2": -1.0}, "expected_return": 0.02},
            {"portfolio": {"a4": 0.5, "a5": 0.5}, "expected_return": 0.06},
        ],
        bl.BlackLittermanModel(covariance, pi=prior, P=p, Q=q, tau=0.025),
    )

    confidences = [0.8, 0.3]
    case(
        "Idzorek confidences",
        market_prior,
        [
            {"portfolio": {a: 1.0}, "expected_return": v, "uncertainty": {"type": "confidence", "confidence": c}}
            for (a, v), c in zip(absolute.items(), confidences)
        ],
        bl.BlackLittermanModel(
            covariance, pi=prior, absolute_views=absolute, omega="idzorek", view_confidences=confidences, tau=0.05
        ),
    )

    variances = [0.0004, 0.0025]
    case(
        "given view variances and prior returns",
        {"type": "returns", "returns": given.tolist()},
        [
            {"portfolio": {a: 1.0}, "expected_return": v, "uncertainty": {"type": "variance", "variance": w}}
            for (a, v), w in zip(absolute.items(), variances)
        ],
        bl.BlackLittermanModel(covariance, pi=given, absolute_views=absolute, omega=np.diag(variances), tau=0.05),
    )

    case(
        "a view held with certainty",
        market_prior,
        [{"portfolio": {"a0": 1.0}, "expected_return": 0.08, "uncertainty": {"type": "confidence", "confidence": 1.0}}],
        bl.BlackLittermanModel(
            covariance, pi=prior, absolute_views={"a0": 0.08}, omega="idzorek", view_confidences=[1.0], tau=0.05
        ),
    )

    rng = np.random.default_rng(20261107)
    prices = pd.Series(100 * np.cumprod(1 + 0.0004 + 0.01 * rng.standard_normal(500)))
    aversion = bl.market_implied_risk_aversion(prices, frequency=252, risk_free_rate=0.02)

    fixture = {
        "generated_by": "scripts/reference/black_litterman.py",
        "labels": labels,
        "covariance": cov.ravel().tolist(),
        "cases": cases,
        "implied_risk_aversion": {
            "returns": prices.pct_change().dropna().tolist(),
            "periods_per_year": 252,
            "risk_free_rate": 0.02,
            "risk_aversion": float(aversion),
        },
    }
    OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
