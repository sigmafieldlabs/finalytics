"""Generate reference values for the finalytics-risk statistics tests.

Return statistics come from empyrical, quantiles from SciPy, and the Cornish-Fisher expected
shortfall from numerical integration of the Edgeworth density, so that each check in
``crates/finalytics-risk/tests/reference.rs`` compares against an independent implementation.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/risk_statistics.py

The output, ``crates/finalytics-risk/tests/fixtures/statistics.json``, is committed. Regenerate it
only when a case is added or a reference library changes, and review the difference.
"""

import json
import pathlib

import empyrical as ep
import numpy as np
from scipy import integrate, stats

OUTPUT = (
    pathlib.Path(__file__).resolve().parents[2]
    / "crates/finalytics-risk/tests/fixtures/statistics.json"
)
CONFIDENCES = [0.95, 0.99]


def per_period(annual, periods):
    return (1.0 + annual) ** (1.0 / periods) - 1.0


def cases():
    rng = np.random.default_rng(20261102)

    daily = 0.0004 + 0.009 * rng.standard_t(4, 756) / np.sqrt(2.0)
    daily_benchmark = 0.0003 + 0.7 * (daily - 0.0004) + 0.004 * rng.standard_normal(756)
    yield "daily", 252.0, 0.04, 0.0, daily, daily_benchmark

    monthly = np.expm1(0.006 + 0.04 * rng.standard_normal(60)) - 0.002
    monthly_benchmark = 0.5 * monthly + 0.02 * rng.standard_normal(60)
    yield "monthly", 12.0, 0.03, 0.05, monthly, monthly_benchmark

    crash = 0.0005 + 0.006 * rng.standard_normal(300)
    crash[120:130] -= 0.03
    crash_benchmark = 0.0002 + 0.008 * rng.standard_normal(300)
    yield "crash", 252.0, 0.0, 0.02, crash, crash_benchmark


def edgeworth_density(x, s, k):
    he3 = x**3 - 3 * x
    he4 = x**4 - 6 * x**2 + 3
    he6 = x**6 - 15 * x**4 + 45 * x**2 - 15
    return stats.norm.pdf(x) * (1 + s / 6 * he3 + k / 24 * he4 + s**2 / 72 * he6)


def tail(returns, confidence):
    alpha = 1.0 - confidence
    mean = np.mean(returns)
    sd = np.std(returns, ddof=1)
    s = stats.skew(returns)
    k = stats.kurtosis(returns)
    z = stats.norm.ppf(alpha)
    h = (
        z
        + (z**2 - 1) * s / 6
        + (z**3 - 3 * z) * k / 24
        - (2 * z**3 - 5 * z) * s**2 / 36
    )
    integral, _ = integrate.quad(
        lambda x: x * edgeworth_density(x, s, k), -np.inf, h, epsabs=1e-14, epsrel=1e-13
    )
    tail_mean = min(integral / alpha, h)
    return {
        "confidence": confidence,
        "historical": [
            -ep.value_at_risk(returns, cutoff=alpha),
            -ep.conditional_value_at_risk(returns, cutoff=alpha),
        ],
        "parametric": [
            -(mean + sd * z),
            -(mean - sd * stats.norm.pdf(z) / alpha),
        ],
        "cornish_fisher": [-(mean + sd * h), -(mean + sd * tail_mean)],
    }


def case(name, periods, risk_free, target, returns, benchmark):
    rf = per_period(risk_free, periods)
    mar = per_period(target, periods)
    active = returns - benchmark
    return {
        "name": name,
        "options": {
            "periods_per_year": periods,
            "risk_free_rate": risk_free,
            "target_return": target,
        },
        "returns": returns.tolist(),
        "benchmark": benchmark.tolist(),
        "statistics": {
            "total_return": float(np.prod(1 + returns) - 1),
            "annualised_return": ep.annual_return(returns, annualization=periods),
            "volatility": ep.annual_volatility(returns, annualization=periods),
            "downside_deviation": ep.downside_risk(
                returns, required_return=mar, annualization=periods
            ),
            "sharpe_ratio": ep.sharpe_ratio(returns, risk_free=rf, annualization=periods),
            "sortino_ratio": ep.sortino_ratio(
                returns, required_return=mar, annualization=periods
            ),
            "max_drawdown": ep.max_drawdown(returns),
            "calmar_ratio": ep.calmar_ratio(returns, annualization=periods),
            "skewness": float(stats.skew(returns)),
            "excess_kurtosis": float(stats.kurtosis(returns)),
        },
        "relative": {
            "active_return": ep.annual_return(returns, annualization=periods)
            - ep.annual_return(benchmark, annualization=periods),
            "tracking_error": float(np.std(active, ddof=1) * np.sqrt(periods)),
            "information_ratio": ep.excess_sharpe(returns, benchmark) * np.sqrt(periods),
            "beta": ep.beta(returns, benchmark, risk_free=rf),
            "alpha": ep.alpha(returns, benchmark, risk_free=rf, annualization=periods),
            "correlation": float(np.corrcoef(returns, benchmark)[0, 1]),
        },
        "tail": [tail(returns, c) for c in CONFIDENCES],
    }


def main():
    probabilities = [1e-300, 1e-50, 1e-10, 1e-4, 0.01, 0.02425, 0.05, 0.3, 0.5, 0.7, 0.975, 0.999]
    fixture = {
        "generated_by": "scripts/reference/risk_statistics.py",
        "normal_quantile": [[p, float(stats.norm.ppf(p))] for p in probabilities],
        "cases": [case(*c) for c in cases()],
    }
    OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(OUTPUT.parents[4])}")


if __name__ == "__main__":
    main()
