#!/usr/bin/env python3
"""Fetch the screening test data: the S&P 500's companies, their daily prices for five years and
their fundamentals, for the screening tests and the README's sample report.

    python3 scripts/fetch-screening-data.py [output folder, default tests/screening]

Needs Python 3.10 or later and nothing else. Writes:

    constituents.csv  the S&P 500 list (symbol, name, sector, sub-industry), from the public-domain
                      dataset at github.com/datasets/s-and-p-500-companies
    prices.csv        id,date,close,volume: daily closes adjusted for dividends and splits, and
                      volumes, from Yahoo Finance's chart data
    fields.csv        id,name,sector,industry,exchange,market_cap,trailing_pe,forward_pe,
                      price_to_book,trailing_dividend_yield,eps_ttm: fundamentals from Yahoo
                      Finance's quotes (empty where Yahoo has no value; trailing_dividend_yield is
                      a fraction, 0.021 for 2.1%)

Answers are kept in <output>/.cache, so a run that stops (Yahoo can refuse requests for a while)
resumes where it stopped. Delete the cache to fetch everything again.
"""

from __future__ import annotations

import csv
import http.cookiejar
import io
import json
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

CONSTITUENTS = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv"
CHART = "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=5y&interval=1d&events=div%2Csplit&includeAdjustedClose=true"
QUOTE = "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}&crumb={}"
AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36"
PAUSE = 0.5  # seconds between calls to Yahoo

jar = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))


def get(url: str, tries: int = 6) -> bytes:
    """The body at `url`, waiting longer each time Yahoo answers 429 or fails."""
    wait = 30
    for attempt in range(tries):
        request = urllib.request.Request(url, headers={"User-Agent": AGENT, "Accept": "*/*"})
        try:
            with opener.open(request, timeout=30) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code == 404:
                raise
            if error.code != 429 and error.code < 500:
                raise
            print(f"  {error.code} from {url.split('?')[0]}; waiting {wait} s", file=sys.stderr)
        except urllib.error.URLError as error:
            print(f"  {error.reason}; waiting {wait} s", file=sys.stderr)
        if attempt < tries - 1:
            time.sleep(wait)
            wait = min(wait * 2, 600)
    raise SystemExit(f"Gave up on {url.split('?')[0]}; run the script again later to resume.")


def yahoo_symbol(symbol: str) -> str:
    return symbol.replace(".", "-")  # BRK.B on the list is BRK-B on Yahoo


def constituents(out: Path) -> list[dict[str, str]]:
    text = get(CONSTITUENTS).decode("utf-8")
    rows = list(csv.DictReader(io.StringIO(text)))
    with open(out / "constituents.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["symbol", "name", "sector", "industry"])
        for r in rows:
            w.writerow([r["Symbol"], r["Security"], r["GICS Sector"], r["GICS Sub-Industry"]])
    print(f"{len(rows)} companies")
    return rows


def history(symbol: str, cache: Path) -> list[tuple[str, float, int | None]]:
    """(date, adjusted close, volume) for each trading day, oldest first."""
    path = cache / f"chart-{yahoo_symbol(symbol)}.json"
    if not path.exists():
        try:
            body = get(CHART.format(yahoo_symbol(symbol)))
        except urllib.error.HTTPError:
            print(f"  no prices for {symbol}", file=sys.stderr)
            path.write_text("{}")
            return []
        path.write_bytes(body)
        time.sleep(PAUSE)
    data = json.loads(path.read_text() or "{}")
    result = ((data.get("chart") or {}).get("result") or [None])[0]
    if not result:
        return []
    offset = result["meta"].get("gmtoffset") or 0
    stamps = result.get("timestamp") or []
    quote = result["indicators"]["quote"][0]
    adjusted = (result["indicators"].get("adjclose") or [{}])[0].get("adjclose") or quote["close"]
    rows: dict[str, tuple[float, int | None]] = {}
    for i, stamp in enumerate(stamps):
        close = adjusted[i] if i < len(adjusted) else None
        if close is None:
            continue
        day = datetime.fromtimestamp(stamp + offset, tz=timezone.utc).strftime("%Y-%m-%d")
        volume = quote["volume"][i] if i < len(quote["volume"]) else None
        rows[day] = (close, int(volume) if volume is not None else None)
    return [(d, c, v) for d, (c, v) in sorted(rows.items())]


def crumb() -> str | None:
    """Yahoo's token for quote requests, from a cookie set by fc.yahoo.com."""
    try:
        opener.open(urllib.request.Request("https://fc.yahoo.com", headers={"User-Agent": AGENT}), timeout=30)
    except urllib.error.HTTPError:
        pass  # it answers 404 but sets the cookie
    try:
        token = get("https://query1.finance.yahoo.com/v1/test/getcrumb").decode().strip()
    except (urllib.error.HTTPError, SystemExit):
        return None
    return token if token and "<" not in token and " " not in token else None


def fundamentals(symbols: list[str], cache: Path) -> dict[str, dict]:
    path = cache / "quotes.json"
    found: dict[str, dict] = json.loads(path.read_text()) if path.exists() else {}
    missing = [s for s in symbols if yahoo_symbol(s) not in found]
    if missing:
        token = crumb()
        if token is None:
            print("Yahoo gave no quote token; fields.csv has names and sectors only. Run again later.", file=sys.stderr)
            return found
        for i in range(0, len(missing), 50):
            chunk = [yahoo_symbol(s) for s in missing[i : i + 50]]
            body = json.loads(get(QUOTE.format(",".join(chunk), token)))
            for q in body.get("quoteResponse", {}).get("result", []):
                found[q["symbol"]] = q
            path.write_text(json.dumps(found))
            time.sleep(PAUSE)
    return found


def number(value) -> str:
    return "" if value is None else f"{value:.6g}"


def main() -> None:
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "tests/screening")
    cache = out / ".cache"
    cache.mkdir(parents=True, exist_ok=True)
    companies = constituents(out)

    with open(out / "prices.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["id", "date", "close", "volume"])
        for n, company in enumerate(companies, 1):
            symbol = company["Symbol"]
            rows = history(symbol, cache)
            for day, close, volume in rows:
                w.writerow([symbol, day, f"{close:.6g}", "" if volume is None else volume])
            if n % 25 == 0:
                print(f"prices: {n} of {len(companies)}")

    quotes = fundamentals([c["Symbol"] for c in companies], cache)
    with open(out / "fields.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["id", "name", "sector", "industry", "exchange", "market_cap", "trailing_pe", "forward_pe",
                    "price_to_book", "trailing_dividend_yield", "eps_ttm"])
        for c in companies:
            q = quotes.get(yahoo_symbol(c["Symbol"]), {})
            w.writerow([
                c["Symbol"], c["Security"], c["GICS Sector"], c["GICS Sub-Industry"], q.get("fullExchangeName", ""),
                number(q.get("marketCap")), number(q.get("trailingPE")), number(q.get("forwardPE")),
                number(q.get("priceToBook")), number(q.get("trailingAnnualDividendYield")),
                number(q.get("epsTrailingTwelveMonths")),
            ])
    print(f"Wrote {out}/constituents.csv, prices.csv and fields.csv")


if __name__ == "__main__":
    main()
