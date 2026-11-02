"""Generate reference values for the scenario tests.

Scenarios on the shared portfolio in ``tests/scenario``, computed independently of the Rust code
with pandas, NumPy and SciPy: the holdings are found by replaying the transactions, the shocks
and historical replays are applied to them, factor shocks are propagated with the conditional
mean of a multivariate normal (``numpy.linalg.pinv``), and risk is the volatility and parametric
value at risk of the weights before and after, with the stressed covariance
``s^2 ((1 - a) C + a v v')``. ``crates/finalytics/tests/scenarios.rs`` compares them.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/scenarios.py

The output, ``crates/finalytics/tests/fixtures/scenarios.json``, is committed. Regenerate it only
when a case is added, and review the difference.
"""

import json
import pathlib

import numpy as np
import pandas as pd
from scipy.stats import norm

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCENARIO = ROOT / "tests/scenario"
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/scenarios.json"
AS_OF = "2026-10-30"

instruments = pd.read_csv(SCENARIO / "instruments.csv").set_index("id")
transactions = pd.read_csv(SCENARIO / "transactions.csv")
prices = pd.read_csv(SCENARIO / "prices.csv", index_col="date")
eurusd = pd.read_csv(SCENARIO / "fx.csv", index_col="date")["EUR/USD"]


def holdings_on(date):
    """Quantities and cash balances after every transaction up to ``date``."""
    quantity, cash = {}, {}

    def move(currency, amount):
        cash[currency] = cash.get(currency, 0.0) + amount

    for t in transactions[transactions.trade_date <= date].itertuples():
        if t.type == "deposit":
            move(t.currency, t.amount)
        elif t.type == "withdrawal":
            move(t.currency, -t.amount)
        elif t.type in ("buy", "sell"):
            sign = 1 if t.type == "buy" else -1
            currency = instruments.loc[t.instrument, "currency"]
            fees = 0.0 if pd.isna(t.fees) else t.fees
            quantity[t.instrument] = quantity.get(t.instrument, 0.0) + sign * t.quantity
            move(currency, -sign * t.quantity * t.price - fees)
        elif t.type == "fx_conversion":
            move(t.sell_currency, -t.sell_amount)
            move(t.buy_currency, t.buy_amount)
        elif t.type == "dividend":
            tax = 0.0 if pd.isna(t.withholding_tax) else t.withholding_tax
            move(t.currency, t.amount - tax)
        else:
            raise ValueError(f"unexpected transaction type {t.type}")
    return quantity, cash


def rate(currency, date):
    return 1.0 if currency == "USD" else float(eurusd.loc[date])


def held(date):
    """Each holding's currency and value in USD, in the engine's holding order: instruments by
    identifier, then cash by currency."""
    quantity, cash = holdings_on(date)
    rows = []
    for i in sorted(quantity):
        currency = instruments.loc[i, "currency"]
        rows.append((i, currency, quantity[i] * prices.loc[date, i] * rate(currency, date)))
    for c in sorted(cash):
        rows.append((f"{c} cash", c, cash[c] * rate(c, date)))
    return rows


def outcome(rows, local, currency, group_by, risk=None):
    """The scenario's result from each holding's local and currency return."""
    before = np.array([v for _, _, v in rows])
    total = (1 + np.array(local)) * (1 + np.array(currency)) - 1
    after = before * (1 + total)
    out = {
        "value_before": before.sum(),
        "value_after": after.sum(),
        "holdings": [
            {"label": name, "return_local": l, "currency_return": c, "return_base": r, "value_after": a}
            for (name, _, _), l, c, r, a in zip(rows, local, currency, total, after)
        ],
        "groups": {},
    }
    for scheme in group_by:
        groups = {}
        for (name, _, b), a in zip(rows, after):
            if name.endswith(" cash"):
                g = "cash"
            else:
                g = instruments.loc[name, f"class.{scheme}"]
            entry = groups.setdefault(g, [0.0, 0.0])
            entry[0] += b
            entry[1] += a
        out["groups"][scheme] = [
            {"group": g, "change": a - b, "contribution": (a - b) / before.sum()} for g, (b, a) in sorted(groups.items())
        ]
    if risk is not None:
        labels, cov, confidence, (scale, shift) = risk
        index = [labels.index(name) if name in labels else None for name, _, _ in rows]

        def sub(matrix):
            n = len(rows)
            m = np.zeros((n, n))
            for i in range(n):
                for j in range(n):
                    if index[i] is not None and index[j] is not None:
                        m[i, j] = matrix[index[i], index[j]]
            return m

        sd = np.sqrt(np.diag(cov))
        stressed = scale**2 * ((1 - shift) * cov + shift * np.outer(sd, sd))
        np.fill_diagonal(stressed, scale**2 * np.diag(cov))
        z = norm.ppf(confidence)
        for key, weights, matrix, value in (
            ("risk_before", before / before.sum(), sub(cov), before.sum()),
            ("risk_after", after / after.sum(), sub(stressed), after.sum()),
        ):
            vol = np.sqrt(weights @ matrix @ weights)
            out[key] = {
                "volatility": vol,
                "value_at_risk": z * vol * value,
                "components": (weights * (matrix @ weights) / vol).tolist(),
            }
    return out


def shock_case():
    rows = held(AS_OF)
    returns = pd.DataFrame(
        {
            "SPY": prices["SPY"],
            "SAP": prices["SAP"] * eurusd,
            "BOND": prices["BOND"],
            "EUR cash": eurusd,
        }
    ).pct_change().dropna()
    labels = list(returns.columns)
    cov = returns.cov().to_numpy()
    shocks = {
        "instruments": {"SAP": -0.35},
        "groups": [
            {"scheme": "asset_class", "value": "Equity", "shock": -0.25},
            {"scheme": "asset_class", "value": "Fixed income", "shock": 0.02},
        ],
        "currencies": {"EUR": -0.08},
    }
    by_instrument = {"SAP": -0.35}
    by_group = {"Equity": -0.25, "Fixed income": 0.02}
    local, currency = [], []
    for name, cur, _ in rows:
        fx = -0.08 if cur == "EUR" else 0.0
        if name.endswith(" cash"):
            local.append(0.0)
        elif name in by_instrument:
            local.append(by_instrument[name])
        else:
            local.append(by_group[instruments.loc[name, "class.asset_class"]])
        currency.append(fx)
    stress = {"volatility_scale": 1.5, "correlation_shift": 0.5}
    options = {
        "group_by": ["asset_class", "region"],
        "risk": {
            "covariance": {"labels": labels, "values": cov.ravel().tolist()},
            "confidence": 0.99,
            "stress": stress,
        },
    }
    expected = outcome(rows, local, currency, options["group_by"], (labels, cov, 0.99, (1.5, 0.5)))
    return {"as_of": AS_OF, "shocks": shocks, "options": options, "expected": expected}


FACTORS = ["equity", "europe", "rates"]
FACTOR_COVARIANCE = np.array(
    [
        [0.0400, 0.0060, -0.0016],
        [0.0060, 0.0090, -0.0003],
        [-0.0016, -0.0003, 0.0001],
    ]
)
EXPOSURES = {"SPY": [1.0, 0.0, 0.0], "SAP": [1.1, 0.8, 0.0], "BOND": [0.0, 0.0, -6.5]}


def factor_case(given, propagate):
    rows = held(AS_OF)
    f = np.zeros(len(FACTORS))
    s = [FACTORS.index(k) for k in given]
    f[s] = list(given.values())
    if propagate:
        u = [i for i in range(len(FACTORS)) if i not in s]
        f[u] = FACTOR_COVARIANCE[np.ix_(u, s)] @ np.linalg.pinv(FACTOR_COVARIANCE[np.ix_(s, s)]) @ f[s]
    local = [0.0 if name.endswith(" cash") else float(np.dot(EXPOSURES[name], f)) for name, _, _ in rows]
    currency = [0.0] * len(rows)
    model = {
        "assets": list(EXPOSURES),
        "exposures": list(EXPOSURES.values()),
        "factor_covariance": {"labels": FACTORS, "values": FACTOR_COVARIANCE.ravel().tolist()},
        "specific_variances": [0.01, 0.02, 0.0001],
    }
    shocks = {"factors": {"model": model, "shocks": dict(given), "propagate": propagate}}
    expected = outcome(rows, local, currency, [])
    expected["factor_returns"] = f.tolist()
    return {"as_of": AS_OF, "shocks": shocks, "options": {}, "expected": expected}


def historical_case(start, end, proxies):
    rows = held(AS_OF)
    window = prices.loc[start:end]
    fx = eurusd.loc[start:end]
    levels = []
    for name, cur, _ in rows:
        series = proxies.get(name, name)
        rate_path = fx if cur == "EUR" else pd.Series(1.0, index=fx.index)
        price_path = pd.Series(1.0, index=window.index) if name.endswith(" cash") else window[series]
        levels.append((price_path / price_path.iloc[0], rate_path / rate_path.iloc[0]))
    local = [float(p.iloc[-1] - 1) for p, _ in levels]
    currency = [float(r.iloc[-1] - 1) for _, r in levels]
    expected = outcome(rows, local, currency, [])
    path = sum(v * p * r for (_, _, v), (p, r) in zip(rows, levels))
    drawdown = path / path.cummax() - 1
    trough = drawdown.idxmin()
    peak = path.loc[:trough].idxmax()
    expected["path"] = {
        "dates": list(path.index),
        "values": path.tolist(),
        "peak": peak,
        "trough": trough,
        "depth": float(drawdown.min()),
    }
    window_input = {"start": start, "end": end, "proxies": dict(proxies)}
    return {"as_of": AS_OF, "window": window_input, "options": {}, "expected": expected}


def main():
    fixture = {
        "generated_by": "scripts/reference/scenarios.py",
        "shocks": shock_case(),
        "factors_one": factor_case({"equity": -0.2}, propagate=True),
        "factors_two": factor_case({"equity": -0.2, "europe": -0.05}, propagate=True),
        "factors_alone": factor_case({"equity": -0.2}, propagate=False),
        "historical": historical_case("2026-09-03", "2026-09-30", {}),
        "historical_proxy": historical_case("2026-09-03", "2026-09-30", {"SAP": "SPY"}),
    }

    def plain(value):
        if isinstance(value, dict):
            return {k: plain(v) for k, v in value.items()}
        if isinstance(value, (list, tuple)):
            return [plain(v) for v in value]
        if isinstance(value, np.generic):
            return value.item()
        return value

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(plain(fixture), indent=1) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
