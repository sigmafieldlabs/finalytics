"""Generate reference values for the household tests.

Performance by account and by group of accounts for the shared portfolio in ``tests/scenario``,
computed independently of the Rust code with pandas and SciPy: each account's values and flows
are found by replaying its transactions on every price date, its time-weighted return is the
product of its daily returns with flows at the start of the day, its money-weighted return is the
root of its net present value (``scipy.optimize.brentq``), and its contribution is each day's
gain over the household's starting value for the day, linked by the household's growth.
``crates/finalytics/tests/households.rs`` compares them.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/households.py

The output, ``crates/finalytics/tests/fixtures/households.json``, is committed. Regenerate it
only when a case is added, and review the difference.
"""

import json
import pathlib

import numpy as np
import pandas as pd
from scipy.optimize import brentq

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCENARIO = ROOT / "tests/scenario"
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/households.json"

accounts = pd.read_csv(SCENARIO / "accounts.csv")
instruments = pd.read_csv(SCENARIO / "instruments.csv").set_index("id")
transactions = pd.read_csv(SCENARIO / "transactions.csv")
prices = pd.read_csv(SCENARIO / "prices.csv", index_col="date")
eurusd = pd.read_csv(SCENARIO / "fx.csv", index_col="date")["EUR/USD"]
DATES = [d for d in prices.index if d >= transactions.trade_date.min()]


def rate(currency, date):
    return 1.0 if currency == "USD" else float(eurusd.loc[date])


def series(account):
    """The account's value at the end of each date and its external flows since the previous
    date, in USD."""
    values, flows = [], []
    quantity, cash = {}, {}
    rows = transactions[transactions.account == account].sort_values("trade_date", kind="stable")
    done = 0
    for date in DATES:
        flow = 0.0
        for t in rows.iloc[done:].itertuples():
            if t.trade_date > date:
                break
            done += 1

            def move(currency, amount):
                cash[currency] = cash.get(currency, 0.0) + amount

            if t.type == "deposit":
                move(t.currency, t.amount)
                flow += t.amount * rate(t.currency, t.trade_date)
            elif t.type == "withdrawal":
                move(t.currency, -t.amount)
                flow -= t.amount * rate(t.currency, t.trade_date)
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
        value = sum(
            q * prices.loc[date, i] * rate(instruments.loc[i, "currency"], date) for i, q in quantity.items()
        )
        value += sum(a * rate(c, date) for c, a in cash.items())
        values.append(value)
        flows.append(flow)
    return np.array(values), np.array(flows)


def daily_returns(values, flows):
    """Each day's return with flows at the start of the day; zero on a day with nothing
    invested."""
    out = []
    for t in range(1, len(values)):
        base = values[t - 1] + flows[t]
        out.append(values[t] / base - 1 if base > 0 else 0.0)
    return np.array(out)


def money_weighted(values, flows):
    origin = pd.Timestamp(DATES[0])
    years = [(pd.Timestamp(d) - origin).days / 365 for d in DATES]
    cash_flows = [(0.0, -values[0])]
    for t in range(1, len(DATES)):
        if flows[t] != 0:
            cash_flows.append((max(years[t] - 1 / 365, 0.0), -flows[t]))
    cash_flows.append((years[-1], values[-1]))

    def npv(r):
        return sum(a / (1 + r) ** y for y, a in cash_flows)

    return brentq(npv, -0.9999, 10_000.0, xtol=1e-15, rtol=4 * np.finfo(float).eps, maxiter=500)


def main():
    lines = {a: series(a) for a in accounts.id}
    household_values = sum(v for v, _ in lines.values())
    household_flows = sum(f for _, f in lines.values())
    household_returns = daily_returns(household_values, household_flows)
    growth = np.concatenate([[1.0], np.cumprod(1 + household_returns)])
    bases = household_values[:-1] + household_flows[1:]

    def measure(members):
        values = sum(lines[a][0] for a in members)
        flows = sum(lines[a][1] for a in members)
        gains = values[1:] - values[:-1] - flows[1:]
        contribution = float(np.sum(np.where(bases > 0, gains / bases, 0.0) * growth[:-1]))
        return {
            "accounts": list(members),
            "value_start": values[0],
            "value_end": values[-1],
            "flows": float(flows[1:].sum()),
            "time_weighted": float(np.prod(1 + daily_returns(values, flows)) - 1),
            "money_weighted": money_weighted(values, flows),
            "contribution": contribution,
        }

    groups = {}
    for scheme in ("owner", "wrapper"):
        names = sorted(accounts[f"class.{scheme}"].unique())
        groups[scheme] = [
            {"name": g, **measure(list(accounts.id[accounts[f"class.{scheme}"] == g]))} for g in names
        ]
    fixture = {
        "generated_by": "scripts/reference/households.py",
        "dates": DATES,
        "household": measure(list(accounts.id)),
        "accounts": [{"name": a, **measure([a])} for a in accounts.id],
        "groups": groups,
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
