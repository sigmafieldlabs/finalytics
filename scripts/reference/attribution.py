"""Generate reference values for the attribution tests.

There is no widely used Python library for Brinson-Fachler attribution, so this script implements
the published formulas directly in NumPy, independently of the Rust code: Brinson and Fachler
(1985) for the effects, Carino (1999) and Menchero (2000) for linking, and Karnosky and Singer
(1994) for currency attribution. ``crates/finalytics-perf/tests/attribution.rs`` compares them.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/attribution.py

The output, ``crates/finalytics-perf/tests/fixtures/attribution.json``, is committed. Regenerate it
only when a case is added, and review the difference.
"""

import json
import pathlib

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics-perf/tests/fixtures/attribution.json"
NAMES = ["equities", "bonds", "property", "cash"]
DATES = ["2026-01-01", "2026-04-01", "2026-07-01", "2026-10-01", "2027-01-01"]


def weights(rng, n):
    w = rng.uniform(0.1, 1.0, n)
    return w / w.sum()


def brinson(w, r, b, s):
    bench = b @ s
    return {
        "portfolio_return": float(w @ r),
        "benchmark_return": float(bench),
        "allocation": ((w - b) * (s - bench)).tolist(),
        "selection": (b * (r - s)).tolist(),
        "interaction": ((w - b) * (r - s)).tolist(),
    }


def carino(rt, bt):
    r, b = np.prod(1 + rt) - 1, np.prod(1 + bt) - 1
    k = (np.log1p(r) - np.log1p(b)) / (r - b)
    kt = (np.log1p(rt) - np.log1p(bt)) / (rt - bt)
    return kt / k


def menchero(rt, bt):
    n = len(rt)
    r, b = np.prod(1 + rt) - 1, np.prod(1 + bt) - 1
    m = (r - b) / n / ((1 + r) ** (1 / n) - (1 + b) ** (1 / n))
    d = rt - bt
    return m + (r - b - m * d.sum()) / (d @ d) * d


def linked(periods, coefficients):
    out = {"coefficients": coefficients.tolist()}
    for key in ("allocation", "selection", "interaction"):
        out[key] = sum(c * np.array(p[key]) for p, c in zip(periods, coefficients)).tolist()
    return out


def brinson_case(rng):
    periods, inputs = [], []
    for k in range(4):
        w, b = weights(rng, 4), weights(rng, 4)
        r = rng.normal(0.02, 0.04, 4)
        s = r - rng.normal(0.0, 0.01, 4)
        inputs.append(
            {
                "start": DATES[k],
                "end": DATES[k + 1],
                "segments": [
                    {
                        "name": NAMES[i],
                        "portfolio_weight": w[i],
                        "portfolio_return": r[i],
                        "benchmark_weight": b[i],
                        "benchmark_return": s[i],
                    }
                    for i in range(4)
                ],
            }
        )
        periods.append(brinson(w, r, b, s))
    rt = np.array([p["portfolio_return"] for p in periods])
    bt = np.array([p["benchmark_return"] for p in periods])
    return {
        "periods_input": inputs,
        "periods": periods,
        "portfolio_return": float(np.prod(1 + rt) - 1),
        "benchmark_return": float(np.prod(1 + bt) - 1),
        "carino": linked(periods, carino(rt, bt)),
        "menchero": linked(periods, menchero(rt, bt)),
    }


CURRENCIES = ["USD", "EUR", "JPY"]


def karnosky_singer(rng, hedged):
    inputs, periods = [], []
    for k in range(4):
        cur = ["USD", "EUR", "JPY", "EUR"]
        w, b = weights(rng, 4), weights(rng, 4)
        r = rng.normal(0.02, 0.04, 4)
        s = r - rng.normal(0.0, 0.01, 4)
        spot = {"USD": 0.0, "EUR": rng.normal(0, 0.03), "JPY": rng.normal(0, 0.04)}
        cash = {"USD": 0.012, "EUR": 0.008, "JPY": 0.001}
        c = np.array([cash[x] for x in cur])
        e = np.array([spot[x] for x in cur])
        f = {x: (1 + cash[x]) * (1 + spot[x]) - 1 for x in CURRENCIES}
        x, y = r - c, s - c
        market = brinson(w, x, b, y)
        if hedged:
            u = dict(zip(CURRENCIES, [0.7, 0.2, 0.1]))
            v = dict(zip(CURRENCIES, [0.5, 0.3, 0.2]))
        else:
            u = {z: float(sum(w[i] for i in range(4) if cur[i] == z)) for z in CURRENCIES}
            v = {z: float(sum(b[i] for i in range(4) if cur[i] == z)) for z in CURRENCIES}
        big_f = sum(v[z] * f[z] for z in CURRENCIES)
        currency = {z: (u[z] - v[z]) * (f[z] - big_f) for z in CURRENCIES}
        model_p = market["portfolio_return"] + sum(u[z] * f[z] for z in CURRENCIES)
        model_b = market["benchmark_return"] + big_f
        if hedged:
            actual_p, actual_b = model_p, model_b
        else:
            actual_p = float(w @ ((1 + r) * (1 + e) - 1))
            actual_b = float(b @ ((1 + s) * (1 + e) - 1))
        period = {
            "start": DATES[k],
            "end": DATES[k + 1],
            "segments": [
                {
                    "name": NAMES[i],
                    "currency": cur[i],
                    "portfolio_weight": w[i],
                    "portfolio_return": r[i],
                    "benchmark_weight": b[i],
                    "benchmark_return": s[i],
                }
                for i in range(4)
            ],
            "currencies": [{"currency": z, "spot_return": spot[z], "cash_return": cash[z]} for z in CURRENCIES],
        }
        if hedged:
            period["portfolio_exposures"] = {z: u[z] for z in CURRENCIES}
            period["benchmark_exposures"] = {z: v[z] for z in CURRENCIES}
        inputs.append(period)
        periods.append(
            {
                **{key: market[key] for key in ("allocation", "selection", "interaction")},
                "currency": [currency[z] for z in CURRENCIES],
                "portfolio_return": actual_p,
                "benchmark_return": actual_b,
                "residual": (actual_p - actual_b) - (model_p - model_b),
            }
        )
    rt = np.array([p["portfolio_return"] for p in periods])
    bt = np.array([p["benchmark_return"] for p in periods])
    coefficients = carino(rt, bt)
    linked_effects = linked(periods, coefficients)
    linked_effects["currency"] = sum(c * np.array(p["currency"]) for p, c in zip(periods, coefficients)).tolist()
    linked_effects["residual"] = float(sum(c * p["residual"] for p, c in zip(periods, coefficients)))
    return {
        "periods_input": inputs,
        "periods": periods,
        "portfolio_return": float(np.prod(1 + rt) - 1),
        "benchmark_return": float(np.prod(1 + bt) - 1),
        "carino": linked_effects,
    }


def main():
    rng = np.random.default_rng(20261108)
    fixture = {
        "generated_by": "scripts/reference/attribution.py",
        "brinson_fachler": brinson_case(rng),
        "karnosky_singer": karnosky_singer(rng, hedged=False),
        "karnosky_singer_hedged": karnosky_singer(rng, hedged=True),
    }

    def plain(value):
        if isinstance(value, dict):
            return {k: plain(v) for k, v in value.items()}
        if isinstance(value, list):
            return [plain(v) for v in value]
        if isinstance(value, np.generic):
            return value.item()
        return value

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(plain(fixture), indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
