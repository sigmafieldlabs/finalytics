"""Generate reference backtests for the finalytics backtest tests.

Each case is simulated afresh here, from the backtest's definition rather than the engine's code,
on the asset-class dataset with T-bills as cash: weights drift with returns; at the end of each
calendar quarter (or year, or when a weight drifts beyond its band) they trade back to the target;
a trade costs the given basis points of the value traded, paid out of the next period's return;
walk-forward validation refits at each calendar rebalance on the window before it. Covariance is
scikit-learn's Ledoit-Wolf estimate, annualised by 12, and the minimum-variance portfolios are
solved in CVXPY.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/backtest.py

The output, ``crates/finalytics/tests/fixtures/backtest.json``, is committed. Regenerate it only
when a case is added or a reference library changes, and review the difference.
"""

import json
import pathlib

import cvxpy as cp
import numpy as np
import pandas as pd
from sklearn.covariance import LedoitWolf

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/backtest.json"
TIGHT = {"tol_gap_abs": 1e-11, "tol_gap_rel": 1e-11, "tol_feas": 1e-11}

prices = pd.read_csv(ROOT / "crates/finalytics/data/asset-classes.csv", index_col="date")
returns = prices / prices.shift(1) - 1
returns = returns.iloc[1:]
DATES = list(returns.index)
RISKY = [c for c in returns.columns if c != "T-Bills"]
X = returns[RISKY].to_numpy()
CASH = returns["T-Bills"].to_numpy()
N, K = X.shape


def covariance(rows):
    return LedoitWolf().fit(X[rows]).covariance_ * 12


def equal_weight(rows):
    return np.full(K, 1 / K)


def inverse_volatility(rows):
    inverse = 1 / np.sqrt(np.diag(covariance(rows)))
    return inverse / inverse.sum()


def minimum_variance(rows, cap=0.4):
    sigma = covariance(rows)
    w = cp.Variable(K)
    problem = cp.Problem(cp.Minimize(cp.quad_form(w, cp.psd_wrap(sigma))), [cp.sum(w) == 1, w >= 0, w <= cap])
    problem.solve(solver=cp.CLARABEL, **TIGHT)
    return np.clip(w.value, 0, None)


def weights_of(mapping):
    return np.array([mapping.get(a, 0.0) for a in RISKY])


def ends_period(schedule, t):
    if t + 1 >= N:
        return False
    a, b = DATES[t], DATES[t + 1]
    ya, ma, yb, mb = int(a[:4]), int(a[5:7]), int(b[:4]), int(b[5:7])
    if schedule == "monthly":
        return (ya, ma) != (yb, mb)
    if schedule == "quarterly":
        return (ya, (ma - 1) // 3) != (yb, (mb - 1) // 3)
    if schedule == "yearly":
        return ya != yb
    return False


def run(rows, start, schedule, bps, band=None, refit=None):
    """Hold `start` over `rows`, trading as the schedule says; `refit(t)` gives new targets."""
    held = np.array(start, dtype=float)
    cash = 1 - held.sum()
    target = held.copy()
    pending = 0.0
    out, trades = [], []
    for t in rows:
        gross = held @ X[t] + cash * CASH[t]
        out.append((1 + gross) * (1 - pending) - 1)
        pending = 0.0
        held = held * (1 + X[t]) / (1 + gross)
        cash = cash * (1 + CASH[t]) / (1 + gross)
        if t == rows[-1]:
            break
        calendar = ends_period(schedule, t)
        refitted = False
        if calendar and refit is not None:
            target = refit(t)
            refitted = True
        target_cash = 1 - target.sum()
        drifted = band is not None and (
            np.abs(target - held).max() > band or abs(target_cash - cash) > band
        )
        if not (calendar or drifted):
            continue
        traded = np.abs(target - held).sum()
        turnover = 0.5 * (traded + abs(target_cash - cash))
        if turnover > 1e-12 or refitted:
            pending = bps / 1e4 * traded
            trades.append({
                "date": DATES[t],
                "weights": dict(zip(RISKY, map(float, target))),
                "turnover": float(turnover),
                "cost": float(pending),
                "refit": refitted,
            })
        held = target.copy()
        cash = target_cash
    return {
        "dates": [DATES[t] for t in rows],
        "returns": [float(r) for r in out],
        "start": dict(zip(RISKY, map(float, start))),
        "trades": trades,
    }


FITS = {"equal_weight": equal_weight, "inverse_volatility": inverse_volatility,
        "minimum_variance": minimum_variance}


def case(name, strategies, validation, schedule, bps, band=None, benchmark=None):
    if validation["type"] == "in_sample":
        parts = {"in": range(N), "out": None, "fit": range(N)}
    elif validation["type"] == "holdout":
        h = next(i for i, d in enumerate(DATES) if d >= validation["start"])
        parts = {"in": range(h), "out": range(h, N), "fit": range(h)}
    else:
        w = round(validation["years"] * 12)
        parts = {"in": range(w, N), "out": range(w, N), "fit": range(N), "window": w,
                 "expanding": validation.get("expanding", False)}
    results = []
    for strategy in strategies:
        if strategy["type"] == "weights":
            start = weights_of(strategy["weights"])
            ins = run(parts["in"], start, schedule, bps, band)
            out = run(parts["out"], start, schedule, bps, band) if parts["out"] else None
        else:
            fit = FITS[strategy["options"]["objective"]]
            if validation["type"] == "walk_forward":
                w, expanding = parts["window"], parts["expanding"]
                first = fit(range(0, w))
                out = run(parts["out"], first, schedule, bps, band,
                          refit=lambda t: fit(range(0 if expanding else t + 1 - w, t + 1)))
                ins = run(parts["in"], fit(range(N)), schedule, bps, band)
            else:
                start = fit(parts["fit"])
                ins = run(parts["in"], start, schedule, bps, band)
                out = run(parts["out"], start, schedule, bps, band) if parts["out"] else None
        results.append({"in_sample": ins, "out_of_sample": out})
    expected = {"strategies": results, "benchmark": None}
    if benchmark is not None:
        start = weights_of(benchmark["weights"])
        expected["benchmark"] = {
            "in_sample": run(parts["in"], start, schedule, bps, band),
            "out_of_sample": run(parts["out"], start, schedule, bps, band) if parts["out"] else None,
        }
    options = {"strategies": strategies, "validation": validation, "trading_cost_bps": bps,
               "rebalance": {"type": "drift", "band": band} if band is not None else {"type": schedule}}
    if benchmark is not None:
        options["benchmark"] = benchmark
    return {"name": name, "options": options, "expected": expected}


def optimise(objective, **extra):
    return {"type": "optimise", "options": {"objective": objective, **extra}}


LIMITED = {"bounds": {"min": 0.0, "max": 0.4}}
cases = [
    case("in sample, quarterly, 10 bp",
         [optimise("equal_weight"), optimise("inverse_volatility"),
          {"type": "weights", "name": "Cautious",
           "weights": {"US Equity": 0.4, "Bonds": 0.4, "T-Bills": 0.2}}],
         {"type": "in_sample"}, "quarterly", 10),
    case("holdout from November 2016, drift of 5 points, 25 bp",
         [optimise("minimum_variance", **LIMITED),
          {"type": "weights", "name": "60/40", "weights": {"US Equity": 0.6, "Bonds": 0.4}}],
         {"type": "holdout", "start": "2016-11-01"}, None, 25, band=0.05,
         benchmark={"type": "weights", "name": "Balanced",
                    "weights": {"US Equity": 0.3, "Intl Equity": 0.2, "Bonds": 0.4, "Gold": 0.1}}),
    case("walk-forward on 5 years, quarterly, 10 bp",
         [optimise("equal_weight"), optimise("inverse_volatility"),
          optimise("minimum_variance", **LIMITED)],
         {"type": "walk_forward", "years": 5}, "quarterly", 10),
    case("walk-forward on all history after 10 years, yearly, no costs",
         [optimise("minimum_variance", **LIMITED)],
         {"type": "walk_forward", "years": 10, "expanding": True}, "yearly", 0),
    case("in sample, never rebalanced",
         [{"type": "weights", "name": "60/40", "weights": {"US Equity": 0.6, "Bonds": 0.4}}],
         {"type": "in_sample"}, "never", 0),
]
OUTPUT.write_text(json.dumps({"cases": cases}, indent=1) + "\n")
print(f"wrote {OUTPUT.relative_to(ROOT)}: {len(cases)} cases")
