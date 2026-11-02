"""Generate reference values for the report tests.

The figures the reports derive themselves, for the shared portfolio in ``tests/scenario``,
computed independently of the Rust code with pandas and NumPy: the portfolio's daily values and
flows by replaying the transactions; its time-weighted returns by month, by calendar year and
over the standard periods, with the benchmark's; external flows and income by month; the
concentration of its holdings; the histogram of its daily returns (``numpy.histogram``); and,
for the attribution report, each segment's weight and return in each month from the holdings'
daily gains, and the Brinson-Fachler effects. ``crates/finalytics/tests/reports_reference.rs``
compares them.

A purchase moves its cost, fees included, from cash into the instrument; a sale moves its
proceeds back; a dividend is the instrument's income, received in cash; a currency conversion
moves the market value of what was sold from one currency's cash to the other's. These are the
engine's documented conventions for contribution (``docs/methods/contribution.md``).

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/reports.py

The output, ``crates/finalytics/tests/fixtures/reports.json``, is committed. Regenerate it only
when a case is added, and review the difference.
"""

import json
import pathlib
from collections import defaultdict

import numpy as np
import pandas as pd

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCENARIO = ROOT / "tests/scenario"
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/reports.json"

instruments = pd.read_csv(SCENARIO / "instruments.csv").set_index("id")
transactions = pd.read_csv(SCENARIO / "transactions.csv")
prices = pd.read_csv(SCENARIO / "prices.csv", index_col="date")
eurusd = pd.read_csv(SCENARIO / "fx.csv", index_col="date")["EUR/USD"]
DATES = [d for d in prices.index if d >= transactions.trade_date.min()]
AS_OF = DATES[-1]
BENCHMARK = "SPY"


def rate(currency, date):
    return 1.0 if currency == "USD" else float(eurusd.loc[date])


def lines():
    """Each holding's value, trades and income in USD on each date, and the portfolio's
    external flows, by replaying the transactions in order."""
    keys = [*instruments.index, "USD cash", "EUR cash"]
    value = {k: [] for k in keys}
    trades = {k: [0.0] * len(DATES) for k in keys}
    income = {k: [0.0] * len(DATES) for k in keys}
    flows = [0.0] * len(DATES)
    quantity, cash = defaultdict(float), defaultdict(float)
    rows = transactions.sort_values("trade_date", kind="stable")
    done = 0
    for t_index, date in enumerate(DATES):
        for t in rows.iloc[done:].itertuples():
            if t.trade_date > date:
                break
            done += 1
            fx = lambda c: rate(c, t.trade_date)  # noqa: E731
            if t.type in ("deposit", "withdrawal"):
                sign = 1 if t.type == "deposit" else -1
                cash[t.currency] += sign * t.amount
                trades[f"{t.currency} cash"][t_index] += sign * t.amount * fx(t.currency)
                flows[t_index] += sign * t.amount * fx(t.currency)
            elif t.type in ("buy", "sell"):
                sign = 1 if t.type == "buy" else -1
                c = instruments.loc[t.instrument, "currency"]
                fees = 0.0 if pd.isna(t.fees) else t.fees
                moved = sign * t.quantity * t.price + fees
                quantity[t.instrument] += sign * t.quantity
                cash[c] -= moved
                trades[f"{c} cash"][t_index] -= moved * fx(c)
                trades[t.instrument][t_index] += moved * fx(c)
            elif t.type == "fx_conversion":
                cash[t.sell_currency] -= t.sell_amount
                cash[t.buy_currency] += t.buy_amount
                moved = t.sell_amount * fx(t.sell_currency)
                trades[f"{t.sell_currency} cash"][t_index] -= moved
                trades[f"{t.buy_currency} cash"][t_index] += moved
            elif t.type == "dividend":
                tax = 0.0 if pd.isna(t.withholding_tax) else t.withholding_tax
                net = (t.amount - tax) * fx(t.currency)
                cash[t.currency] += t.amount - tax
                income[t.instrument][t_index] += net
                trades[f"{t.currency} cash"][t_index] += net
            else:
                raise ValueError(f"unexpected transaction type {t.type}")
        for i in instruments.index:
            value[i].append(quantity[i] * prices.loc[date, i] * rate(instruments.loc[i, "currency"], date))
        for c in ("USD", "EUR"):
            value[f"{c} cash"].append(cash[c] * rate(c, date))
    return keys, value, trades, income, flows


def monthly(dates, levels):
    """Monthly and calendar-year returns of index levels, each from the last level of the
    period before (or the first)."""
    s = pd.Series(levels, index=pd.to_datetime(dates))
    ends = s.groupby([s.index.year, s.index.month]).tail(1)
    anchors = pd.concat([s.iloc[:1], ends]).iloc[:-1].to_numpy()
    months = {f"{d.year:04}-{d.month:02}": v / a - 1 for d, v, a in zip(ends.index, ends.to_numpy(), anchors)}
    year_ends = s.groupby(s.index.year).tail(1)
    year_anchors = pd.concat([s.iloc[:1], year_ends]).iloc[:-1].to_numpy()
    years = {str(d.year): v / a - 1 for d, v, a in zip(year_ends.index, year_ends.to_numpy(), year_anchors)}
    return months, years


def main():
    keys, value, trades, income, flows = lines()
    values = np.array([sum(value[k][t] for k in keys) for t in range(len(DATES))])
    flows = np.array(flows)
    daily = [values[t] / (values[t - 1] + flows[t]) - 1 for t in range(1, len(DATES))]
    growth = np.concatenate([[1.0], np.cumprod(1 + np.array(daily))])
    months, years = monthly(DATES, growth)
    bench = prices[BENCHMARK].loc[DATES].to_numpy()
    bench_months, bench_years = monthly(DATES, bench)

    def period(start_index):
        return {
            "start": DATES[start_index],
            "cumulative": growth[-1] / growth[start_index] - 1,
            "benchmark": bench[-1] / bench[start_index] - 1,
        }

    last_september = max(i for i, d in enumerate(DATES) if d <= "2026-09-30")
    periods = {"MTD": period(last_september), "QTD": period(last_september), "ITD": period(0)}

    flows_by_month = defaultdict(lambda: {"inflows": 0.0, "outflows": 0.0, "income": 0.0})
    for t in transactions.itertuples():
        month = t.trade_date[:7]
        if t.type in ("deposit", "withdrawal"):
            amount = t.amount * rate(t.currency, t.trade_date)
            flows_by_month[month]["inflows" if t.type == "deposit" else "outflows"] += (
                amount if t.type == "deposit" else -amount
            )
        elif t.type == "dividend":
            tax = 0.0 if pd.isna(t.withholding_tax) else t.withholding_tax
            flows_by_month[month]["income"] += (t.amount - tax) * rate(t.currency, t.trade_date)

    held = {i: value[i][-1] for i in instruments.index if abs(value[i][-1]) > 0}
    invested = sum(held.values())
    shares = sorted((v / values[-1] for v in held.values()), reverse=True)
    herfindahl = sum((v / invested) ** 2 for v in held.values())

    counts, edges = np.histogram(np.array(daily), bins=30)

    # Attribution: segments by asset class, cash as "cash", over September and October.
    segment = {i: instruments.loc[i, "class.asset_class"] for i in instruments.index}
    segment.update({"USD cash": "cash", "EUR cash": "cash"})
    benchmark_periods = [
        ("2026-09-01", "2026-09-30", {"Equity": (0.6, -0.04), "Fixed income": (0.4, 0.01)}),
        ("2026-09-30", "2026-10-30", {"Equity": (0.6, -0.02), "Fixed income": (0.4, 0.005)}),
    ]
    index = {d: k for k, d in enumerate(DATES)}
    attribution = []
    for start, end, bench_segments in benchmark_periods:
        i, j = index[start], index[end]
        names = sorted(set(segment.values()))
        start_value = {g: sum(value[k][i] for k in keys if segment[k] == g) for g in names}
        total = sum(start_value.values())
        growth_by = {g: 1.0 for g in names}
        for t in range(i + 1, j + 1):
            for g in names:
                members = [k for k in keys if segment[k] == g]
                gain = sum(value[k][t] - value[k][t - 1] - trades[k][t] + income[k][t] for k in members)
                base = sum(value[k][t - 1] + max(trades[k][t], 0.0) for k in members)
                if base > 0:
                    growth_by[g] *= 1 + gain / base
        segs = []
        for name, (b, s) in bench_segments.items():
            w = start_value.get(name, 0.0) / total
            r = growth_by[name] - 1 if w != 0 else s
            segs.append((name, w, r, b, s))
        for name in names:
            if name not in bench_segments and start_value[name] != 0:
                r = growth_by[name] - 1
                segs.append((name, start_value[name] / total, r, 0.0, r))
        R = sum(w * r for _, w, r, _, _ in segs)
        B = sum(b * s for _, _, _, b, s in segs)
        attribution.append(
            {
                "start": start,
                "end": end,
                "segments": [
                    {
                        "name": name,
                        "portfolio_weight": w,
                        "portfolio_return": r,
                        "allocation": (w - b) * (s - B),
                        "selection": b * (r - s),
                        "interaction": (w - b) * (r - s),
                    }
                    for name, w, r, b, s in segs
                ],
            }
        )

    fixture = {
        "generated_by": "scripts/reference/reports.py",
        "as_of": AS_OF,
        "months": months,
        "years": years,
        "benchmark_months": bench_months,
        "benchmark_years": bench_years,
        "periods": periods,
        "flows": dict(sorted(flows_by_month.items())),
        "concentration": {
            "top_five": float(sum(shares[:5])),
            "herfindahl": herfindahl,
            "effective_holdings": 1 / herfindahl,
        },
        "histogram": {"counts": counts.tolist(), "lower": float(edges[0]), "upper": float(edges[-1])},
        "attribution": attribution,
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
