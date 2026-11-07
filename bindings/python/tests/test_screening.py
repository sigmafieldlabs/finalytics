"""Screens: a universe from a long DataFrame, a mapping of Series or CSV, with fields; rules,
sort and limit from plain keywords; the reference fixture; and the results as a DataFrame."""

import json
import math

import numpy as np
import pandas as pd
import pytest

import finalytics as fl
import finalytics.pandas as flpd
from conftest import FIXTURES, NO_BROWSER, SP500, page_path

FIXTURE = json.loads((FIXTURES / "finalytics/tests/fixtures/screening.json").read_text())


@pytest.fixture(scope="module")
def prices() -> pd.DataFrame:
    return pd.read_csv(SP500 / "prices.csv", index_col="date", parse_dates=True)


@pytest.fixture(scope="module")
def companies() -> pd.DataFrame:
    return pd.read_csv(SP500 / "companies.csv", index_col="ticker")


@pytest.fixture(scope="module")
def universe(prices, companies) -> fl.Universe:
    """The fixture's universe: the S&P 500 companies of the recipe, cut to their own dates."""
    index = pd.read_csv(SP500 / "index.csv", index_col="date", parse_dates=True)
    rows = []
    fields = {}
    for s, r in enumerate(FIXTURE["recipe"]):
        close = prices[r["id"]].dropna()
        if "start" in r:
            close = close[close.index >= r["start"]]
        if "end" in r:
            close = close[close.index <= r["end"]]
        for a, b in r.get("gaps", []):
            close = close[(close.index < a) | (close.index > b)]
        k = np.arange(len(close))
        frame = pd.DataFrame({"id": r["id"], "date": close.index, "close": close.to_numpy()})
        frame["volume"] = 1_000_000.0 + ((k * 7919 + s * 104729) % 1000) * 1000.0 if r.get("volume") else np.nan
        frame.loc[frame["date"].isin(pd.to_datetime(r.get("volume_missing", []))), "volume"] = np.nan
        frame["dividends"] = np.where((k + s) % 63 == 0, 0.25 * (1 + s % 3), 0.0) if r.get("dividends") else np.nan
        frame.loc[frame["date"].isin(pd.to_datetime(r.get("missing", []))), "close"] = np.nan
        rows.append(frame)
        fields[r["id"]] = {}
        if r["id"] not in FIXTURE["no_sector"]:
            fields[r["id"]]["sector"] = companies.loc[r["id"], "sector"]
        if r["id"] not in FIXTURE["no_pe"]:
            fields[r["id"]]["pe_ratio"] = 8.0 + (s * 37) % 29
    rows.append(pd.DataFrame({"id": "S&P 500", "date": index.index, "close": index["S&P 500"].to_numpy()}))
    long = pd.concat(rows, ignore_index=True).sample(frac=1.0, random_state=7)
    # Dividends are given only for the companies the recipe says; a NaN elsewhere is none.
    return fl.Universe(long, fields=fields, benchmark=FIXTURE["benchmark"], risk_free_rate=FIXTURE["risk_free_rate"])


def test_a_screen_matches_the_reference(universe):
    # The rows came in a random order: each security keeps its own, sorted by date.
    assert len(universe) == 21 and set(universe.ids) == {s["id"] for s in FIXTURE["cases"][0]["securities"]}
    for case in FIXTURE["cases"][1:]:
        o = case["options"]
        rules = [
            (r.get("measure", r.get("field")), r["op"], r.get("values", r.get("value")))
            for r in o.get("rules", [])
        ]
        sort = o.get("sort")
        screen = universe.screen(
            as_of=o.get("as_of"),
            max_carry_days=o.get("max_carry_days", 5),
            measures=o.get("measures"),
            rules=rules,
            sort=None if sort is None else (sort.get("measure", sort.get("field")), sort["order"]),
            limit=o.get("limit"),
        )
        r = screen.results
        assert r.matches == case["matches"], case["name"]
        assert list(r.measures) == case["measures"]
        assert [(x.passed, x.lacking, x.remaining) for x in r.rules] == [
            (x["passed"], x["lacking"], x["remaining"]) for x in case["rules"]
        ]
        c = case["coverage"]
        assert (r.coverage.lacking, r.coverage.lacking_history) == (c["lacking"], c["lacking_history"])
        by_id = {s.id: s for s in r.securities}
        for e in case["securities"]:
            s = by_id[e["id"]]
            assert (s.status, s.passed, s.rank) == (e["status"], e["passed"], e["rank"]), e["id"]
            for m, a, x in zip(r.measures, s.values, e["values"]):
                if x is None:
                    assert a is None, (e["id"], m)
                else:
                    assert math.isclose(a, x, rel_tol=1e-9, abs_tol=1e-9), (e["id"], m, a, x)


def test_a_screen_prints_shows_and_becomes_a_dataframe(universe, tmp_path, monkeypatch):
    screen = universe.screen(
        rules=[("return_1y", ">", 0), ("sector", "not_in", ["Utilities"]), ("rsi_14", "between", (20, 80))],
        sort=("sharpe_ratio", "descending"),
        limit=5,
        name="Momentum",
    )
    text = str(screen)
    assert "FINALYTICS  ·  SCREEN" in text and "Momentum" in text
    assert all(len(line) <= 80 for line in text.splitlines())
    assert "ScreenResults" in str(screen.results)
    assert screen.report.to_html().startswith("<!doctype html>")
    tables = [t.name for t in fl.reports.tables(screen.report)]
    assert tables == ["coverage", "rules", "matches", "near_misses", "spread"]
    frame = flpd.securities(screen)
    assert frame.index.name == "id" and len(frame) == 21
    assert list(frame[frame.matched].sort_values("rank").index) == screen.results.matches
    assert frame.loc["DUK", "status"] == "stale"
    assert math.isnan(frame.loc["GE", "return_1y"])
    assert frame.loc["AAPL", "sector"] == "Information Technology"
    assert str(frame["rank"].dtype) == "Int64"


@NO_BROWSER
def test_show_opens_the_page(universe, monkeypatch):
    opened = []
    monkeypatch.setattr("webbrowser.open", lambda url: opened.append(url) or True)
    path = universe.screen(measures=["return_1y"]).show()
    assert path is not None and page_path(opened[0]) == path
    assert path.read_text(encoding="utf-8").startswith("<!doctype html>")


def test_universes_from_a_mapping_and_from_csv(prices, tmp_path):
    closes = {name: prices[name].dropna().iloc[-400:] for name in ["AAPL", "MSFT", "XOM"]}
    closes["XOM"] = closes["XOM"].iloc[:-20]
    u = fl.Universe(closes, fields={"AAPL": {"sector": "Tech"}}, risk_free_rate=0.02)
    assert u.ids == ["AAPL", "MSFT", "XOM"] and u.fields == ["sector"] and u.risk_free_rate == 0.02
    r = u.screen().results
    assert [s.status for s in r.securities] == ["measured", "measured", "stale"]
    path = tmp_path / "prices.csv"
    prices[["AAPL", "MSFT"]].iloc[-300:].to_csv(path)
    fields = tmp_path / "fields.csv"
    fields.write_text("id,sector,pe_ratio\nAAPL,Tech,31.2\nMSFT,Tech,\n")
    from_csv = fl.Universe(path, fields=fields, benchmark="MSFT")
    assert from_csv.benchmark == "MSFT" and from_csv.fields == ["sector", "pe_ratio"]
    screen = from_csv.screen(rules=[("pe_ratio", "<", 40)])
    assert screen.results.matches == ["AAPL"]
    frame = pd.DataFrame({"pe_ratio": [31.2, np.nan], "sector": ["Tech", None]}, index=["AAPL", "MSFT"])
    assert fl.Universe(path, fields=frame).screen(rules=[("sector", "in", ["Tech"])]).results.matches == ["AAPL"]


def test_problems_are_reported_together(universe):
    with pytest.raises(fl.ValidationError) as found:
        universe.screen(
            as_of="2026-06-30",
            measures=["return_2y"],
            rules=[("returns_1y", ">", 0), ("sector", ">", 1), ("rsi_14", "between", (70, 30))],
            sort=("sector", "descending"),
            limit=0,
        )
    paths = [i.path for i in found.value.issues]
    assert paths == ["measures[0]", "rules[0].field", "rules[1].op", "rules[2].value", "sort.field", "limit"]
    with pytest.raises(fl.ValidationError) as found:
        fl.Universe("id,date,close\nA,2026-10-30,1\n", benchmark="SPY", risk_free_rate=-2.0)
    assert [i.path for i in found.value.issues] == ["risk_free_rate", "benchmark"]
    with pytest.raises(fl.ValidationError) as found:
        universe.screen(sort="sharpe_ratio")
    assert found.value.issues[0].path == "sort"
