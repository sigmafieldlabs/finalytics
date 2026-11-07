"""Generate reference screens for the finalytics screening tests.

The universe is built from the S&P 500 prices in ``tests/sp500``: twenty companies and the
S&P 500 total return index as the benchmark, each cut to its own dates so that the histories are
uneven. One company starts in 2022, one in 2024, one in April 2026 and one ten days before the
end; one stops two weeks before the end (stale); one has two gaps of several weeks; and one has
closes that are not numbers, which are no closes. Volumes and dividends are synthetic, from a
fixed formula of the row and the company; one company lacks volumes on a recent day. The fields
are each company's GICS sector (text) and a synthetic price-to-earnings ratio (numbers), missing
for some.

Each measure is computed afresh here from its definition, with pandas: calendar windows from
``pandas.DateOffset``, returns from ``pct_change``, standard deviations with ``ddof=1``,
quantiles with ``numpy.quantile``, exponential averages with ``ewm(adjust=False)``, and beta and
correlation from ``numpy.cov`` and ``numpy.corrcoef`` on the dates both have a close. The rules,
the sort and the limit are applied to the values found, and each case records every security's
values, status and rank, the matches, each rule's counts and the coverage.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/screening.py

The output, ``crates/finalytics/tests/fixtures/screening.json``, is committed. It holds the
recipe of the universe, which the tests rebuild from ``tests/sp500``, and the expected results.
Regenerate it only when a case is added or a reference library changes, and review the
difference.
"""

import json
import math
import pathlib

import numpy as np
import pandas as pd

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/screening.json"
PRICES = pd.read_csv(ROOT / "tests/sp500/prices.csv", index_col="date", parse_dates=True)
INDEX = pd.read_csv(ROOT / "tests/sp500/index.csv", index_col="date", parse_dates=True)
COMPANIES = pd.read_csv(ROOT / "tests/sp500/companies.csv", index_col="ticker")

DAYS = 252
RISK_FREE = 0.03
BENCHMARK = "S&P 500"

# Each company: where its history starts and ends, gaps, closes that are not numbers, and
# whether it has volumes and dividends.
RECIPE = [
    {"id": "AAPL", "volume": True, "dividends": True},
    {"id": "MSFT", "volume": True, "dividends": True},
    {"id": "XOM", "volume": True, "dividends": True},
    {"id": "NEE", "volume": True, "dividends": True},
    {"id": "JPM", "start": "2022-01-03", "volume": True},
    {"id": "NVDA", "start": "2024-03-15", "volume": True},
    {"id": "LLY", "start": "2026-04-01", "volume": True, "dividends": True},
    {"id": "GE", "start": "2026-10-19"},
    {"id": "DUK", "end": "2026-10-15", "volume": True, "dividends": True},
    {"id": "CAT", "gaps": [["2025-12-20", "2026-01-10"], ["2026-06-01", "2026-07-15"]], "volume": True},
    {"id": "KO", "missing": ["2026-03-02", "2026-03-03", "2026-10-28"], "dividends": True},
    {"id": "WMT", "volume": True, "volume_missing": ["2026-10-26"]},
    {"id": "AMZN", "volume": True},
    {"id": "HD", "dividends": True},
    {"id": "PG", "volume": True, "dividends": True},
    {"id": "COST"},
    {"id": "GS", "volume": True},
    {"id": "MU", "volume": True},
    {"id": "SO", "volume": True, "dividends": True},
    {"id": "LIN", "volume": True},
]
NO_SECTOR = {"LIN"}
NO_PE = {"JPM", "AMZN"}


def volume_of(k: int, s: int) -> float:
    """The synthetic volume of row `k` of the `s`-th company."""
    return 1_000_000.0 + ((k * 7919 + s * 104729) % 1000) * 1000.0


def dividend_of(k: int, s: int) -> float:
    """The synthetic dividend of row `k` of the `s`-th company: one a quarter."""
    return 0.25 * (1 + s % 3) if (k + s) % 63 == 0 else 0.0


def pe_of(s: int) -> float:
    return 8.0 + (s * 37) % 29


class Security:
    def __init__(self, s: int, recipe: dict):
        self.id = recipe["id"]
        close = PRICES[self.id].dropna()
        if "start" in recipe:
            close = close[close.index >= recipe["start"]]
        if "end" in recipe:
            close = close[close.index <= recipe["end"]]
        for a, b in recipe.get("gaps", []):
            close = close[(close.index < a) | (close.index > b)]
        # Volumes and dividends are of each row of the history as given, before closes that are
        # not numbers are left out.
        rows = np.arange(len(close))
        volume = pd.Series([volume_of(k, s) for k in rows], index=close.index)
        for d in recipe.get("volume_missing", []):
            volume[pd.Timestamp(d)] = np.nan
        dividends = pd.Series([dividend_of(k, s) for k in rows], index=close.index)
        keep = ~close.index.isin(pd.to_datetime(recipe.get("missing", [])))
        self.close = close[keep]
        self.volume = volume[keep] if recipe.get("volume") else None
        self.dividends = dividends[keep] if recipe.get("dividends") else None
        self.fields = {}
        if self.id not in NO_SECTOR:
            self.fields["sector"] = COMPANIES.loc[self.id, "sector"]
        if self.id not in NO_PE:
            self.fields["pe_ratio"] = pe_of(s)


def benchmark() -> "Security":
    b = Security.__new__(Security)
    b.id = BENCHMARK
    b.close = INDEX[BENCHMARK].dropna()
    b.volume = None
    b.dividends = None
    b.fields = {}
    return b


UNIVERSE = [Security(s, r) for s, r in enumerate(RECIPE)] + [benchmark()]

# Every measure, in the order the engine lists them, with the history it needs: a calendar
# window, or a number of closes.
MEASURES = [
    ("return_1w", ("window", "1w")),
    ("return_1m", ("window", "1m")),
    ("return_3m", ("window", "3m")),
    ("return_6m", ("window", "6m")),
    ("return_ytd", ("window", "ytd")),
    ("return_1y", ("window", "1y")),
    ("return_3y", ("window", "3y")),
    ("return_5y", ("window", "5y")),
    ("volatility", ("window", "1y")),
    ("volatility_3y", ("window", "3y")),
    ("downside_deviation", ("window", "1y")),
    ("max_drawdown", ("window", "1y")),
    ("max_drawdown_3y", ("window", "3y")),
    ("drawdown", ("closes", 1)),
    ("sharpe_ratio", ("window", "1y")),
    ("sharpe_ratio_3y", ("window", "3y")),
    ("sortino_ratio", ("window", "1y")),
    ("calmar_ratio", ("window", "3y")),
    ("value_at_risk", ("window", "1y")),
    ("beta", ("window", "1y")),
    ("correlation", ("window", "1y")),
    ("relative_return_6m", ("window", "6m")),
    ("relative_return_1y", ("window", "1y")),
    ("price_to_sma_20", ("closes", 20)),
    ("price_to_sma_50", ("closes", 50)),
    ("price_to_sma_200", ("closes", 200)),
    ("sma_50_above_200", ("closes", 200)),
    ("rsi_14", ("closes", 15)),
    ("macd_histogram", ("closes", 34)),
    ("from_52w_high", ("window", "1y")),
    ("from_52w_low", ("window", "1y")),
    ("volatility_20d", ("closes", 21)),
    ("dividend_yield", ("window", "1y")),
    ("average_volume_20d", ("closes", 20)),
    ("volume_ratio", ("closes", 20)),
    ("dollar_volume_20d", ("closes", 20)),
    ("observations", ("closes", 0)),
    ("last_price", ("closes", 1)),
    ("stale", ("closes", 0)),
]
NEED = dict(MEASURES)
DATA = {"observations", "last_price", "stale"}


def start_of(period: str, as_of: pd.Timestamp) -> pd.Timestamp:
    """The first day of a trailing calendar window ending on `as_of`."""
    if period == "1w":
        return as_of - pd.Timedelta(days=7)
    if period == "ytd":
        return pd.Timestamp(as_of.year - 1, 12, 31)
    months = {"1m": 1, "3m": 3, "6m": 6, "1y": 12, "3y": 36, "5y": 60}[period]
    return as_of - pd.DateOffset(months=months)


def window(close: pd.Series, start: pd.Timestamp) -> pd.Series | None:
    """The closes from the last on or before `start`, or None if the history starts later."""
    before = close[close.index <= start]
    if before.empty:
        return None
    return close[close.index >= before.index[-1]]


def stats(w: pd.Series | None) -> dict:
    out = dict.fromkeys(["vol", "sharpe", "dd", "sortino", "mdd", "var"])
    if w is None:
        return out
    out["mdd"] = float((w / w.cummax() - 1.0).min())
    r = w.pct_change().dropna()
    if len(r) < 2:
        return out
    rf = (1.0 + RISK_FREE) ** (1.0 / DAYS) - 1.0
    sd = r.std(ddof=1)
    out["vol"] = sd * math.sqrt(DAYS)
    out["sharpe"] = (r.mean() - rf) / sd * math.sqrt(DAYS)
    downside = math.sqrt(float(np.mean(np.minimum(r - rf, 0.0) ** 2)))
    out["dd"] = downside * math.sqrt(DAYS)
    out["sortino"] = (r.mean() - rf) * DAYS / out["dd"] if downside > 0 else None
    out["var"] = -float(np.quantile(r.to_numpy(), 0.05))
    return out


def measure(sec: Security, bench: Security | None, as_of: pd.Timestamp, carry: int) -> dict:
    close = sec.close[sec.close.index <= as_of]
    n = len(close)
    values = dict.fromkeys(NEED)
    values["observations"] = float(n)
    values["stale"] = 0.0
    if n == 0:
        return {"values": values, "stale": False, "n": 0, "first": None, "last": None}
    last = float(close.iloc[-1])
    stale = (as_of - close.index[-1]).days > carry
    values["last_price"] = last
    values["stale"] = 1.0 if stale else 0.0
    found = {"values": values, "stale": stale, "n": n, "first": close.index[0], "last": close.index[-1]}
    if stale:
        return found
    win = {p: window(close, start_of(p, as_of)) for p in ["1w", "1m", "3m", "6m", "ytd", "1y", "3y", "5y"]}

    def ret(p):
        w = win[p]
        return None if w is None else last / float(w.iloc[0]) - 1.0

    for p in ["1w", "1m", "3m", "6m", "ytd", "1y"]:
        values[f"return_{p}"] = ret(p)
    for p, years in [("3y", 3), ("5y", 5)]:
        r = ret(p)
        values[f"return_{p}"] = None if r is None else (1.0 + r) ** (1.0 / years) - 1.0
    one, three = stats(win["1y"]), stats(win["3y"])
    values["volatility"] = one["vol"]
    values["volatility_3y"] = three["vol"]
    values["downside_deviation"] = one["dd"]
    values["max_drawdown"] = one["mdd"]
    values["max_drawdown_3y"] = three["mdd"]
    values["drawdown"] = last / float(close.max()) - 1.0
    values["sharpe_ratio"] = one["sharpe"]
    values["sharpe_ratio_3y"] = three["sharpe"]
    values["sortino_ratio"] = one["sortino"]
    if values["return_3y"] is not None and three["mdd"] is not None and three["mdd"] < 0:
        values["calmar_ratio"] = values["return_3y"] / -three["mdd"]
    values["value_at_risk"] = one["var"]

    if bench is not None:
        b = bench.close[bench.close.index <= as_of]
        if win["1y"] is not None:
            joined = pd.concat([win["1y"], b], axis=1, join="inner").dropna()
            rs = joined.iloc[:, 0].pct_change().dropna().to_numpy()
            rb = joined.iloc[:, 1].pct_change().dropna().to_numpy()
            if len(rs) >= 2:
                values["beta"] = float(np.cov(rs, rb, ddof=1)[0, 1] / np.var(rb, ddof=1))
                values["correlation"] = float(np.corrcoef(rs, rb)[0, 1])
        for p in ["6m", "1y"]:
            wb = window(b, start_of(p, as_of))
            if win[p] is not None and wb is not None:
                values[f"relative_return_{p}"] = ret(p) - (float(b.iloc[-1]) / float(wb.iloc[0]) - 1.0)

    for k in [20, 50, 200]:
        if n >= k:
            values[f"price_to_sma_{k}"] = last / float(close.iloc[-k:].mean())
    if n >= 200:
        values["sma_50_above_200"] = 1.0 if close.iloc[-50:].mean() > close.iloc[-200:].mean() else 0.0
    if n >= 15:
        change = close.diff().dropna().reset_index(drop=True)
        gains, losses = change.clip(lower=0.0), (-change).clip(lower=0.0)
        g = pd.concat([pd.Series([gains.iloc[:14].mean()]), gains.iloc[14:]], ignore_index=True)
        lo = pd.concat([pd.Series([losses.iloc[:14].mean()]), losses.iloc[14:]], ignore_index=True)
        ag = g.ewm(alpha=1 / 14, adjust=False).mean().iloc[-1]
        al = lo.ewm(alpha=1 / 14, adjust=False).mean().iloc[-1]
        if ag + al > 0:
            values["rsi_14"] = 100.0 * ag / (ag + al)
    if n >= 34:
        line = close.ewm(span=12, adjust=False).mean() - close.ewm(span=26, adjust=False).mean()
        signal = line.ewm(span=9, adjust=False).mean()
        values["macd_histogram"] = float(line.iloc[-1] - signal.iloc[-1])
    if win["1y"] is not None:
        values["from_52w_high"] = last / float(win["1y"].max()) - 1.0
        values["from_52w_low"] = last / float(win["1y"].min()) - 1.0
    if n >= 21:
        values["volatility_20d"] = float(close.iloc[-21:].pct_change().dropna().std(ddof=1)) * math.sqrt(DAYS)
    if sec.dividends is not None and win["1y"] is not None:
        start = start_of("1y", as_of)
        d = sec.dividends[(sec.dividends.index > start) & (sec.dividends.index <= as_of)]
        values["dividend_yield"] = float(d.sum()) / last
    if sec.volume is not None and n >= 20:
        v = sec.volume[sec.volume.index <= as_of].iloc[-20:]
        if not v.isna().any():
            average = float(v.mean())
            values["average_volume_20d"] = average
            values["volume_ratio"] = float(v.iloc[-1]) / average if average > 0 else None
            values["dollar_volume_20d"] = float((v * close.iloc[-20:]).mean())
    return found


def covers(name: str, found: dict, as_of: pd.Timestamp) -> bool:
    kind, need = NEED[name]
    if kind == "closes":
        return found["n"] >= need
    return found["n"] > 0 and found["first"] <= start_of(need, as_of)


def case(name: str, options: dict) -> dict:
    as_of = pd.Timestamp(options.get("as_of", "2026-10-30"))
    carry = options.get("max_carry_days", 5)
    rules = options.get("rules", [])
    sort = options.get("sort")
    # The measures computed: those asked for (every one by default), then those the rules and
    # the sort use.
    measures = list(options["measures"]) if "measures" in options else [m for m, _ in MEASURES]
    needed = []
    for target in rules + ([sort] if sort else []):
        if "measure" in target:
            if target["measure"] not in measures:
                measures.append(target["measure"])
            if target["measure"] not in needed:
                needed.append(target["measure"])
    bench = UNIVERSE[-1]
    rows = []
    for sec in UNIVERSE:
        found = measure(sec, bench, as_of, carry)
        if found["stale"]:
            status = "stale"
        elif found["n"] < 2 or not all(covers(m, found, as_of) for m in needed):
            status = "too_short"
        else:
            status = "measured"
        verdicts = []
        for rule in rules:
            if status == "stale":
                verdicts.append("fail")
                continue
            if "measure" in rule:
                x = found["values"][rule["measure"]]
            else:
                x = sec.fields.get(rule["field"])
            if x is None:
                verdicts.append("lacking")
                continue
            op, v = rule["op"], rule.get("value")
            ok = {
                "<": lambda: x < v,
                "<=": lambda: x <= v,
                ">": lambda: x > v,
                ">=": lambda: x >= v,
                "between": lambda: v[0] <= x <= v[1],
                "in": lambda: x in rule["values"],
                "not_in": lambda: x not in rule["values"],
            }[op]()
            verdicts.append("pass" if ok else "fail")
        rows.append({"sec": sec, "found": found, "status": status, "verdicts": verdicts})
    passed = [r["status"] != "stale" and all(v == "pass" for v in r["verdicts"]) for r in rows]
    order = [i for i, p in enumerate(passed) if p]
    if sort:

        def key(i):
            target = sort.get("measure")
            x = rows[i]["found"]["values"][target] if target else rows[i]["sec"].fields.get(sort["field"])
            return (x is None, 0.0 if x is None else (-x if sort["order"] == "descending" else x))

        order.sort(key=key)
    if "limit" in options:
        order = order[: options["limit"]]
    rank = {i: k + 1 for k, i in enumerate(order)}
    live = [r for r in rows if r["status"] != "stale"]
    rule_counts = []
    for k in range(len(rules)):
        rule_counts.append(
            {
                "passed": sum(r["verdicts"][k] == "pass" for r in live),
                "lacking": sum(r["verdicts"][k] == "lacking" for r in live),
                "remaining": sum(all(v == "pass" for v in r["verdicts"][: k + 1]) for r in live),
            }
        )

    def clean(x):
        return None if x is None or (isinstance(x, float) and math.isnan(x)) else float(x)

    def day(d):
        return None if d is None else d.strftime("%Y-%m-%d")

    return {
        "name": name,
        "options": options,
        "measures": measures,
        "matches": [rows[i]["sec"].id for i in order],
        "rules": rule_counts,
        "coverage": {
            "measured": sum(r["status"] == "measured" for r in rows),
            "too_short": sum(r["status"] == "too_short" for r in rows),
            "stale": sum(r["status"] == "stale" for r in rows),
            "passed": sum(passed),
            "matched": len(order),
            "lacking": [sum(r["found"]["values"][m] is None for r in live) for m in measures],
        },
        "securities": [
            {
                "id": r["sec"].id,
                "status": r["status"],
                "passed": passed[i],
                "rank": rank.get(i),
                "first_date": day(r["found"]["first"]),
                "last_date": day(r["found"]["last"]),
                "values": [clean(r["found"]["values"][m]) for m in measures],
            }
            for i, r in enumerate(rows)
        ],
    }


CASES = [
    case("every measure", {}),
    case(
        "returns and fields",
        {
            "as_of": "2025-06-13",
            "max_carry_days": 3,
            "measures": ["return_1y", "volatility", "sharpe_ratio", "rsi_14"],
            "rules": [
                {"measure": "return_1y", "op": ">", "value": 0.0},
                {"field": "sector", "op": "not_in", "values": ["Utilities"]},
                {"field": "pe_ratio", "op": "between", "value": [12.0, 30.0]},
            ],
            "sort": {"measure": "sharpe_ratio", "order": "descending"},
            "limit": 6,
        },
    ),
    case(
        "technicals",
        {
            "rules": [
                {"measure": "rsi_14", "op": "<", "value": 70.0},
                {"measure": "price_to_sma_200", "op": ">", "value": 1.0},
                {"measure": "macd_histogram", "op": ">", "value": 0.0},
                {"measure": "volatility_3y", "op": "<=", "value": 0.45},
            ],
            "sort": {"measure": "volatility", "order": "ascending"},
        },
    ),
    case(
        "income and volume",
        {
            "measures": [],
            "rules": [
                {"measure": "dividend_yield", "op": ">=", "value": 0.002},
                {"measure": "average_volume_20d", "op": ">", "value": 1_000_000.0},
                {"field": "sector", "op": "in", "values": ["Energy", "Utilities", "Consumer Staples", "Health Care"]},
            ],
            "sort": {"field": "pe_ratio", "order": "ascending"},
            "limit": 3,
        },
    ),
]

fixture = {
    "benchmark": BENCHMARK,
    "risk_free_rate": RISK_FREE,
    "recipe": RECIPE,
    "no_sector": sorted(NO_SECTOR),
    "no_pe": sorted(NO_PE),
    "cases": CASES,
}
OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")
print(f"wrote {OUTPUT.relative_to(ROOT)}: {len(CASES)} cases, {len(UNIVERSE)} securities")
