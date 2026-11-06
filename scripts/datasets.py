#!/usr/bin/env python3
"""Write the datasets in crates/finalytics/data that are built rather than downloaded.

- asset-classes.csv: 30 years of month-end levels of six asset-class indices, synthetic: drawn
  from fixed distributions with a fixed seed, with episodes modelled loosely on the bear markets
  of 2000-02, 2007-09, 2020 and 2022. It is not the history of any market or fund;
- portfolio/: three years of a two-account household in US dollars and euros. Its prices,
  distributions and EUR/USD rate are real (scripts/portfolio-market.json, downloaded by
  scripts/fetch-market-data.py); its transactions are illustrative.

Run it from the repository root with NumPy installed, after scripts/fetch-market-data.py:

    python3 scripts/datasets.py
"""

import csv
import datetime as dt
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/finalytics/data"


def business_days(start: dt.date, end: dt.date) -> list[dt.date]:
    days, d = [], start
    while d <= end:
        if d.weekday() < 5:
            days.append(d)
        d += dt.timedelta(days=1)
    return days


def month_ends(days: list[dt.date]) -> list[int]:
    """The index of the last business day of each month."""
    return [i for i in range(len(days)) if i == len(days) - 1 or days[i + 1].month != days[i].month]


def write_csv(path: Path, header: list[str], rows: list[list[object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(header)
        writer.writerows(rows)


def asset_classes() -> None:
    """Month-end index levels of six asset classes, starting at 100."""
    rng = np.random.default_rng(1996)
    days = business_days(dt.date(1996, 10, 1), dt.date(2026, 10, 30))
    dates = [days[i] for i in month_ends(days)]
    months = [(d.year, d.month) for d in dates[1:]]
    n = len(months)

    names = ["US Equity", "Intl Equity", "Bonds", "Gold", "Real Estate"]
    # Annual log growth and volatility over the whole span, crises included.
    growth = np.array([0.085, 0.055, 0.038, 0.060, 0.070])
    vol = np.array([0.155, 0.170, 0.055, 0.155, 0.190])
    corr = np.array([
        [1.00, 0.80, 0.05, 0.05, 0.65],
        [0.80, 1.00, 0.05, 0.15, 0.60],
        [0.05, 0.05, 1.00, 0.25, 0.25],
        [0.05, 0.15, 0.25, 1.00, 0.10],
        [0.65, 0.60, 0.25, 0.10, 1.00],
    ])
    # Episodes: (first month, last month, total log return of each asset over the episode).
    episodes = [
        ((2000, 9), (2002, 9), [-0.55, -0.55, 0.08, 0.05, -0.15]),
        ((2007, 11), (2009, 2), [-0.75, -0.85, 0.05, 0.15, -1.00]),
        ((2009, 3), (2010, 3), [0.45, 0.45, 0.02, 0.05, 0.60]),
        ((2020, 2), (2020, 3), [-0.30, -0.30, 0.02, -0.02, -0.35]),
        ((2020, 4), (2020, 8), [0.35, 0.25, 0.01, 0.10, 0.20]),
        ((2022, 1), (2022, 10), [-0.25, -0.25, -0.17, -0.05, -0.30]),
    ]
    shock = np.zeros((n, len(names)))
    for first, last, total in episodes:
        inside = [k for k, m in enumerate(months) if first <= m <= last]
        shock[inside] += np.array(total) / len(inside)
    drift = growth / 12 - shock.sum(axis=0) / n
    chol = np.linalg.cholesky(np.outer(vol, vol) * corr / 12)
    noise = rng.standard_normal((n, len(names))) @ chol.T
    # Centre the noise, so that each index grows over the whole span exactly as set above.
    log_returns = drift + shock + noise - noise.mean(axis=0)

    # T-bills earn the short rate of the month, which moves between regimes.
    regimes = [
        ((1996, 1), 0.051), ((2001, 1), 0.035), ((2002, 1), 0.016), ((2005, 1), 0.032),
        ((2006, 1), 0.048), ((2008, 1), 0.015), ((2009, 1), 0.001), ((2016, 1), 0.006),
        ((2017, 1), 0.012), ((2018, 1), 0.020), ((2019, 1), 0.021), ((2020, 4), 0.001),
        ((2022, 3), 0.012), ((2022, 9), 0.033), ((2023, 1), 0.050), ((2024, 9), 0.047),
        ((2025, 1), 0.042), ((2026, 1), 0.038),
    ]
    rates = np.array([[r for start, r in regimes if start <= m][-1] for m in months])
    bills = np.log1p(rates / 12 + rng.normal(0.0, 0.0002, n))

    levels = 100 * np.exp(np.vstack([np.zeros(len(names) + 1),
                                     np.cumsum(np.column_stack([log_returns, bills]), axis=0)]))
    order = ["US Equity", "Intl Equity", "Bonds", "T-Bills", "Gold", "Real Estate"]
    column = {name: levels[:, j] for j, name in enumerate(names + ["T-Bills"])}
    rows = [[d.isoformat()] + [f"{column[name][i]:.4f}" for name in order] for i, d in enumerate(dates)]
    write_csv(OUT / "asset-classes.csv", ["date", *order], rows)


def portfolio() -> None:
    """A two-account household over three years, on the real prices, distributions and EUR/USD
    rate in scripts/portfolio-market.json (written by scripts/fetch-market-data.py). The
    transactions are illustrative."""
    market = json.loads((ROOT / "scripts/portfolio-market.json").read_text())
    days = [dt.date.fromisoformat(d) for d in market["dates"]]
    n = len(days)
    held = ["VTI", "EXSA", "BND", "GLD", "VNQ"]

    def aligned(series: dict[str, float]) -> np.ndarray:
        """The last value on or before each US trading day."""
        known = sorted((dt.date.fromisoformat(d), v) for d, v in series.items())
        out, k, last = np.empty(n), 0, None
        for i, day in enumerate(days):
            while k < len(known) and known[k][0] <= day:
                last = known[k][1]
                k += 1
            out[i] = last
        return out

    funds = market["funds"]
    prices = {k: np.round(aligned(funds[k]["close"]), 2) for k in held}
    prices["EURUSD"] = np.round(aligned(market["eur_usd"]), 5)
    total = {k: aligned(funds[k]["adjusted"]) for k in held}
    index = {d: i for i, d in enumerate(days)}
    us_day = {d: next(day for day in days if day >= d) for d in map(dt.date.fromisoformat, (
        x for k in held for x in funds[k]["dividends"]))}
    distributions = sorted((us_day[dt.date.fromisoformat(d)], k, a)
                           for k in held for d, a in funds[k]["dividends"].items())

    # A balanced benchmark from the funds' total returns: 55% equity (70% US, 30% European in
    # dollars), 35% bonds and 10% real assets (half gold, half real estate), rebalanced at each
    # month end.
    exsa_usd = total["EXSA"] * prices["EURUSD"]

    def chained(parts: list[tuple[float, np.ndarray]]) -> np.ndarray:
        daily = sum(w * np.diff(s) / s[:-1] for w, s in parts)
        return np.cumprod(np.r_[1.0, 1.0 + daily])

    segments = {
        "Equity": chained([(0.7, total["VTI"]), (0.3, exsa_usd)]),
        "Fixed income": total["BND"] / total["BND"][0],
        "Real assets": chained([(0.5, total["GLD"]), (0.5, total["VNQ"])]),
    }
    weights = {"Equity": 0.55, "Fixed income": 0.35, "Real assets": 0.10}
    ends = month_ends(days)
    bench = np.empty(n)
    bench[0], anchor = 100.0, 0
    for i in range(1, n):
        bench[i] = bench[anchor] * sum(w * segments[s][i] / segments[s][anchor] for s, w in weights.items())
        if i in ends:
            anchor = i

    out = OUT / "portfolio"
    write_csv(out / "prices.csv", ["date", *held, "BENCH"],
              [[d.isoformat()] + [f"{prices[k][i]:.2f}" for k in held] + [f"{bench[i]:.4f}"]
               for i, d in enumerate(days)])
    write_csv(out / "fx.csv", ["date", "EUR/USD"],
              [[d.isoformat(), f"{prices['EURUSD'][i]:.5f}"] for i, d in enumerate(days)])
    write_csv(out / "instruments.csv", ["id", "kind", "currency", "name", "class.asset_class", "class.region"], [
        ["VTI", "fund", "USD", "Vanguard Total Stock Market ETF", "Equity", "North America"],
        ["EXSA", "fund", "EUR", "iShares STOXX Europe 600 UCITS ETF (DE)", "Equity", "Europe"],
        ["BND", "fund", "USD", "Vanguard Total Bond Market ETF", "Fixed income", "North America"],
        ["GLD", "fund", "USD", "SPDR Gold Shares", "Real assets", "Global"],
        ["VNQ", "fund", "USD", "Vanguard Real Estate ETF", "Real assets", "North America"],
    ])
    write_csv(out / "accounts.csv", ["id", "name", "lot_method", "taxable", "class.owner", "class.wrapper"], [
        ["brokerage", "Brokerage account", "fifo", "true", "Alex", "Taxable"],
        ["ira", "Retirement account", "fifo", "false", "Alex", "Tax-deferred"],
    ])

    rows: list[dict[str, str]] = []
    held_units = {(a, k): 0 for a in ("brokerage", "ira") for k in held}

    def tx(day: dt.date, account: str, kind: str, **fields: str) -> None:
        rows.append({"id": f"t{len(rows) + 1}", "account": account, "trade_date": day.isoformat(),
                     "type": kind, **fields})

    def price(k: str, day: dt.date) -> float:
        return float(prices[k][index[day]])

    def buy(day: dt.date, account: str, k: str, value: float) -> None:
        p = price(k, day)
        q = int(value // p)
        held_units[(account, k)] += q
        tx(day, account, "buy", instrument=k, quantity=str(q), price=f"{p:.2f}", fees="1")

    def buy_european(day: dt.date, account: str, dollars: float) -> None:
        rate, p = price("EURUSD", day), price("EXSA", day)
        q = int(dollars / rate // p)
        euros = round(q * p + 1, 2)
        tx(day, account, "fx_conversion", sell_amount=f"{euros * rate:.2f}", sell_currency="USD",
           buy_amount=f"{euros:.2f}", buy_currency="EUR")
        held_units[(account, "EXSA")] += q
        tx(day, account, "buy", instrument="EXSA", quantity=str(q), price=f"{p:.2f}", fees="1")

    first = days[0]
    tx(first, "brokerage", "deposit", amount="250000", currency="USD")
    tx(first, "ira", "deposit", amount="150000", currency="USD")
    buy(first, "brokerage", "VTI", 100000)
    buy_european(first, "brokerage", 37500)
    buy(first, "brokerage", "BND", 62500)
    buy(first, "brokerage", "GLD", 25000)
    buy(first, "brokerage", "VNQ", 22500)
    buy(first, "ira", "VTI", 75000)
    buy(first, "ira", "BND", 72000)

    events: list[tuple[dt.date, int, str, object]] = []
    for y in range(2023, 2027):
        for m in range(1, 13):
            if (2023, 11) < (y, m) <= (2026, 10):
                events.append((next(d for d in days if (d.year, d.month) == (y, m)), 0, "contribution", None))
    events += [(day, 1, "distribution", (k, a)) for day, k, a in distributions if day > first]
    events += [(dt.date(2025, 2, 3), 2, "rebalance", None), (dt.date(2025, 4, 9), 2, "buy_the_dip", None),
               (dt.date(2025, 6, 2), 2, "fee", None), (dt.date(2026, 1, 15), 2, "withdrawal", None)]
    for day, _, kind, detail in sorted(events, key=lambda e: e[:2]):
        if kind == "contribution":
            tx(day, "brokerage", "deposit", amount="2000", currency="USD")
            buy(day, "brokerage", "VTI", 1990)
        elif kind == "distribution":
            k, per_unit = detail
            for account in ("brokerage", "ira"):
                if q := held_units[(account, k)]:
                    gross = q * per_unit
                    if k == "EXSA":
                        tx(day, account, "dividend", instrument=k, amount=f"{gross:.2f}", currency="EUR",
                           withholding_tax=f"{gross * 0.15:.2f}")
                    else:
                        tx(day, account, "dividend", instrument=k, amount=f"{gross:.2f}", currency="USD")
        elif kind == "rebalance":
            q = int(held_units[("brokerage", "VTI")] * 0.2)
            held_units[("brokerage", "VTI")] -= q
            p = price("VTI", day)
            tx(day, "brokerage", "sell", instrument="VTI", quantity=str(q), price=f"{p:.2f}", fees="1")
            buy(day, "brokerage", "BND", q * p - 10)
        elif kind == "buy_the_dip":
            buy(day, "ira", "VTI", 6000)
        elif kind == "fee":
            tx(day, "brokerage", "fee", amount="250", currency="USD", note="Advisory fee")
        elif kind == "withdrawal":
            tx(day, "brokerage", "withdrawal", amount="10000", currency="USD")

    columns = ["id", "account", "trade_date", "type", "instrument", "quantity", "price", "fees", "amount",
               "currency", "sell_amount", "sell_currency", "buy_amount", "buy_currency", "withholding_tax", "note"]
    write_csv(out / "transactions.csv", columns, [[r.get(c, "") for c in columns] for r in rows])

    periods, previous = [], 0
    for i in ends:
        periods.append({
            "start": days[previous].isoformat(),
            "end": days[i].isoformat(),
            "segments": [{"name": s, "benchmark_weight": w,
                          "benchmark_return": round(float(segments[s][i] / segments[s][previous] - 1), 10)}
                         for s, w in weights.items()],
        })
        previous = i

    # The risk-free rate: the T-bills' annual return over the same three years, from the T-bill
    # index of the S&P 500 test data.
    with open(ROOT / "tests/sp500/prices.csv") as f:
        bills = {r["date"]: float(r["T-Bills"]) for r in csv.DictReader(f)}
    years = (days[-1] - days[0]).days / 365.25
    risk_free = round((bills[days[-1].isoformat()] / bills[days[0].isoformat()]) ** (1 / years) - 1, 3)

    def groups(*shocks: tuple[str, float]) -> list[dict[str, object]]:
        return [{"scheme": "asset_class", "value": v, "shock": s} for v, s in shocks]

    options = {
        "group_by": ["asset_class", "region"],
        "benchmark": "BENCH",
        "risk_free_rate": risk_free,
        "attribution": {"scheme": "asset_class", "periods": periods},
        "scenarios": [
            {"name": "Equity sell-off",
             "scenario": {"type": "shocks", "shocks": {"groups": groups(
                 ("Equity", -0.25), ("Real assets", -0.10), ("Fixed income", 0.03))}},
             "stress": {"volatility_scale": 1.5, "correlation_shift": 0.3}},
            {"name": "Rates up 1%",
             "scenario": {"type": "shocks", "shocks": {"groups": groups(
                 ("Fixed income", -0.06), ("Equity", -0.05))}}},
            {"name": "Dollar up 10%",
             "scenario": {"type": "shocks", "shocks": {"currencies": {"EUR": -0.10}}}},
            {"name": "Spring 2025 replay",
             "scenario": {"type": "historical", "window": {"start": "2025-02-19", "end": "2025-04-08"}}},
        ],
    }
    (out / "report-options.json").write_text(json.dumps(options, indent=2) + "\n")


if __name__ == "__main__":
    asset_classes()
    portfolio()
