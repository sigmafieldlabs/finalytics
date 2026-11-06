#!/usr/bin/env python3
"""Download the market data behind the real datasets in crates/finalytics/data.

- sp500/: daily adjusted closes, from January 2005 to October 2026, of the five largest
  companies in each of the 11 GICS sectors of the S&P 500 that have traded since the start;
  the S&P 500 total return index; and 3-month US Treasury bills as a return index;
- scripts/portfolio-market.json: the quoted closes, adjusted closes and distributions of the
  five funds of the portfolio dataset, and the EUR/USD rate, from which scripts/datasets.py
  writes the portfolio's files;
- us-cpi.csv: the US consumer price index (CPI-U, seasonally adjusted), monthly, from FRED.

Prices come from Yahoo Finance, the price index from the Federal Reserve Bank of St. Louis, and the S&P 500 members and their sectors from the
datasets/s-and-p-500-companies list on GitHub. Members are chosen by their market
capitalisation on the day the script runs, so the selection looks back with today's
knowledge (survivorship bias). It needs only the standard library:

    python3 scripts/fetch-market-data.py
"""

import csv
import datetime as dt
import http.cookiejar
import io
import json
import re
import time
import urllib.parse
import urllib.request
import zoneinfo
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/finalytics/data"
START, END = dt.date(2005, 1, 3), dt.date(2026, 10, 30)
PORTFOLIO_START = dt.date(2023, 11, 1)
PER_SECTOR = 5
MEMBERS = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv"
CPI = "https://fred.stlouisfed.org/graph/fredgraph.csv?id=CPIAUCSL"
FUNDS = {"VTI": "VTI", "EXSA": "EXSA.DE", "BND": "BND", "GLD": "GLD", "VNQ": "VNQ"}

jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
opener.addheaders = [("User-Agent", "Mozilla/5.0")]


def get(url: str) -> bytes:
    for attempt in range(4):
        try:
            with opener.open(url, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code == 404 and "fc.yahoo.com" in url:
                return b""
            if attempt == 3:
                raise
        except OSError:
            if attempt == 3:
                raise
        time.sleep(2 * (attempt + 1))
    raise RuntimeError(url)


def crumb() -> str:
    get("https://fc.yahoo.com")  # sets the session cookie
    return get("https://query2.finance.yahoo.com/v1/test/getcrumb").decode()


def quotes(symbols: list[str], token: str) -> dict[str, dict]:
    out = {}
    for k in range(0, len(symbols), 50):
        batch = ",".join(symbols[k:k + 50])
        url = ("https://query2.finance.yahoo.com/v7/finance/quote?"
               + urllib.parse.urlencode({"symbols": batch, "crumb": token,
                                         "fields": "marketCap,firstTradeDateMilliseconds,longName"}))
        for q in json.loads(get(url))["quoteResponse"]["result"]:
            out[q["symbol"]] = q
        time.sleep(0.3)
    return out


def chart(symbol: str, start: dt.date, end: dt.date) -> dict:
    """Daily closes, adjusted closes and distributions by local trading date."""
    p1 = int(dt.datetime.combine(start, dt.time(), dt.timezone.utc).timestamp())
    p2 = int(dt.datetime.combine(end + dt.timedelta(days=1), dt.time(), dt.timezone.utc).timestamp())
    url = (f"https://query1.finance.yahoo.com/v8/finance/chart/{urllib.parse.quote(symbol)}?"
           f"period1={p1}&period2={p2}&interval=1d&events=div%7Csplit&includeAdjustedClose=true")
    result = json.loads(get(url))["chart"]["result"][0]
    zone = zoneinfo.ZoneInfo(result["meta"]["exchangeTimezoneName"])

    def day(ts: int) -> dt.date:
        return dt.datetime.fromtimestamp(ts, zone).date()

    quote = result["indicators"]["quote"][0]
    adjusted = result["indicators"].get("adjclose", [{}])[0].get("adjclose") or quote["close"]
    rows = {}
    for ts, close, adj in zip(result["timestamp"], quote["close"], adjusted):
        d = day(ts)
        if start <= d <= end and close is not None and adj is not None:
            rows[d] = (close, adj)
    dividends = {day(int(e["date"])): e["amount"]
                 for e in result.get("events", {}).get("dividends", {}).values()}
    time.sleep(0.3)
    return {"currency": result["meta"].get("currency"), "rows": rows,
            "dividends": {d: a for d, a in dividends.items() if start <= d <= end}}


def significant(x: float, digits: int = 6) -> str:
    """`digits` significant figures (more for numbers below one), without an exponent or
    trailing zeros."""
    whole = len(str(int(abs(x)))) if abs(x) >= 1 else 0
    text = f"{x:.{max(0, digits - whole)}f}"
    return text.rstrip("0").rstrip(".") if "." in text else text


def company(name: str) -> str:
    """The list's names, in running order: "Lilly (Eli)" as "Eli Lilly", "Home Depot (The)" as
    "The Home Depot", and "Alphabet Inc. (Class A)" as "Alphabet Inc."."""
    match = re.fullmatch(r"(.*) \((.*)\)", name)
    if not match:
        return name
    if match[2].startswith("Class "):
        return match[1]
    return f"{match[2]} {match[1]}"


def write_csv(path: Path, header: list[str], rows: list[list[object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(header)
        writer.writerows(rows)


def sp500(token: str) -> None:
    members = list(csv.DictReader(io.StringIO(get(MEMBERS).decode())))
    symbols = {m["Symbol"].replace(".", "-"): m for m in members}
    info = quotes(sorted(symbols), token)
    first_day = dt.datetime.combine(START, dt.time(), dt.timezone.utc).timestamp() * 1000
    candidates: dict[str, list[tuple[float, str]]] = {}
    for symbol, member in symbols.items():
        q = info.get(symbol, {})
        if q.get("marketCap") and q.get("firstTradeDateMilliseconds", first_day + 1) <= first_day:
            candidates.setdefault(member["GICS Sector"], []).append((q["marketCap"], symbol))

    chosen: list[tuple[str, str, float]] = []
    series: dict[str, dict] = {}
    companies: set[str] = set()
    for sector in sorted(candidates):
        picked = 0
        for cap, symbol in sorted(candidates[sector], reverse=True):
            if picked == PER_SECTOR:
                break
            if symbols[symbol]["CIK"] in companies:
                continue  # another share class of a company already chosen
            data = chart(symbol, START, END)
            days = sorted(data["rows"])
            # Daily history over the whole span, with no gap of more than a week.
            if not days or days[0] > START + dt.timedelta(days=7) or days[-1] < END - dt.timedelta(days=7):
                continue
            if max((b - a).days for a, b in zip(days, days[1:])) > 7:
                continue
            series[symbol] = data
            companies.add(symbols[symbol]["CIK"])
            chosen.append((symbol, sector, cap))
            picked += 1

    index = chart("^SP500TR", START, END)
    bills = chart("^IRX", START, END)
    dates = sorted(index["rows"])
    # T-bills: the 13-week rate, accrued each calendar day at actual/360 and compounded.
    rate, level, t_bills, previous = None, 100.0, {}, None
    for d in dates:
        if previous is not None and rate is not None:
            level *= 1 + rate / 100 * (d - previous).days / 360
        if d in bills["rows"]:
            rate = bills["rows"][d][0]
        t_bills[d] = level
        previous = d

    def value(symbol: str, d: dt.date, last: dict[str, float]) -> str:
        row = series[symbol]["rows"].get(d)
        if row is not None:
            last[symbol] = row[1]
        return significant(last[symbol])

    last: dict[str, float] = {}
    tickers = [s for s, _, _ in chosen]
    out = OUT / "sp500"
    write_csv(out / "prices.csv", ["date", *tickers, "T-Bills"],
              [[d.isoformat(), *(value(s, d, last) for s in tickers), significant(t_bills[d], 10)] for d in dates])
    write_csv(out / "index.csv", ["date", "S&P 500"],
              [[d.isoformat(), significant(index["rows"][d][1])] for d in dates])
    write_csv(out / "companies.csv", ["ticker", "name", "sector", "industry"],
              [[s, company(symbols[s]["Security"]), sector, symbols[s]["GICS Sub-Industry"]] for s, sector, _ in chosen])
    print(f"{len(chosen)} companies, {len(dates)} days, {dates[0]} to {dates[-1]}")
    for s, sector, cap in chosen:
        print(f"  {sector:24} {s:6} {company(symbols[s]['Security']):32} {cap / 1e9:8.0f}bn")


def portfolio() -> None:
    funds = {k: chart(symbol, PORTFOLIO_START, END) for k, symbol in FUNDS.items()}
    fx = chart("EURUSD=X", PORTFOLIO_START, END)
    us_days = sorted(funds["VTI"]["rows"])
    data = {
        "dates": [d.isoformat() for d in us_days],
        "funds": {k: {"currency": f["currency"],
                      "close": {d.isoformat(): c for d, (c, _) in sorted(f["rows"].items())},
                      "adjusted": {d.isoformat(): a for d, (_, a) in sorted(f["rows"].items())},
                      "dividends": {d.isoformat(): a for d, a in sorted(f["dividends"].items())}}
                  for k, f in funds.items()},
        "eur_usd": {d.isoformat(): c for d, (c, _) in sorted(fx["rows"].items())},
    }
    (ROOT / "scripts/portfolio-market.json").write_text(json.dumps(data, indent=1) + "\n")
    print(f"portfolio: {len(us_days)} US trading days, {us_days[0]} to {us_days[-1]}")


def us_cpi() -> None:
    """The US consumer price index from FRED, dated at each month's end, through the month before
    the last month of the other datasets. A month FRED has no value for is written empty."""
    rows = []
    for line in list(csv.reader(io.StringIO(get(CPI).decode())))[1:]:
        first = dt.date.fromisoformat(line[0])
        if (first.year, first.month) >= (END.year, END.month):
            break
        following = dt.date(first.year + first.month // 12, first.month % 12 + 1, 1)
        value = line[1].strip()
        rows.append([(following - dt.timedelta(days=1)).isoformat(),
                     "" if value in ("", ".") else f"{float(value):.3f}"])
    write_csv(OUT / "us-cpi.csv", ["date", "US CPI"], rows)
    print(f"US CPI: {len(rows)} months, {rows[0][0]} to {rows[-1][0]}")


if __name__ == "__main__":
    sp500(crumb())
    portfolio()
    us_cpi()
